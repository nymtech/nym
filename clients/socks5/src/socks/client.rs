#![forbid(unsafe_code)]

use super::authentication::{AuthenticationMethods, Authenticator, User};
use super::request::{SocksCommand, SocksRequest};
use super::types::{ResponseCode, SocksProxyError};
use super::{RESERVED, SOCKS_VERSION};
use client_core::client::inbound_messages::InputMessage;
use client_core::client::inbound_messages::InputMessageSender;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use pin_project::pin_project;
use proxy_helpers::connection_controller::{
    ConnectionReceiver, ControllerCommand, ControllerSender,
};
use proxy_helpers::proxy_runner::ProxyRunner;
use rand::RngCore;
use socks5_requests::{ConnectionId, Message, RemoteAddress, Request};
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::{self, net::TcpStream};

#[pin_project(project = StateProject)]
enum StreamState {
    Available(TcpStream),
    RunningProxy,
}

impl StreamState {
    fn finish_proxy(&mut self, stream: TcpStream) {
        match self {
            StreamState::RunningProxy => *self = StreamState::Available(stream),
            StreamState::Available(_) => panic!("invalid state!"),
        }
    }

    fn run_proxy(&mut self) -> TcpStream {
        // It's not the nicest way to do it, but it works
        #[allow(unused_assignments)]
        let mut stream = None;
        *self = match std::mem::replace(self, StreamState::RunningProxy) {
            StreamState::Available(inner_stream) => {
                stream = Some(inner_stream);
                StreamState::RunningProxy
            }
            StreamState::RunningProxy => panic!("invalid state"),
        };
        stream.unwrap()
    }

    /// Returns the remote address that this stream is connected to.
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        match self {
            StreamState::RunningProxy => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "stream is being used to run the proxy",
            )),
            StreamState::Available(ref stream) => stream.peer_addr(),
        }
    }

    async fn shutdown(&mut self) -> io::Result<()> {
        // shutdown should only be called if proxy is not being run. If it is, there's some bug
        // somewhere
        match self {
            StreamState::RunningProxy => panic!("Tried to shutdown stream while proxy is running"),
            StreamState::Available(ref mut stream) => TcpStream::shutdown(stream).await,
        }
    }
}

// convenience implementations
impl AsyncRead for StreamState {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.project() {
            StateProject::RunningProxy => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::NotFound,
                "stream is being used to run the proxy",
            ))),
            StateProject::Available(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for StreamState {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.project() {
            StateProject::RunningProxy => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::NotFound,
                "stream is being used to run the proxy",
            ))),
            StateProject::Available(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.project() {
            StateProject::RunningProxy => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::NotFound,
                "stream is being used to run the proxy",
            ))),
            StateProject::Available(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.project() {
            StateProject::RunningProxy => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::NotFound,
                "stream is being used to run the proxy",
            ))),
            StateProject::Available(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// A client connecting to the Socks proxy server, because
/// it wants to make a Nym-protected outbound request. Typically, this is
/// something like e.g. a wallet app running on your laptop connecting to
/// SphinxSocksServer.
pub(crate) struct SocksClient {
    controller_sender: ControllerSender,
    stream: StreamState,
    auth_nmethods: u8,
    authenticator: Authenticator,
    socks_version: u8,
    input_sender: InputMessageSender,
    connection_id: ConnectionId,
    service_provider: Recipient,
    self_address: Recipient,
    started_proxy: bool,
}

impl Drop for SocksClient {
    fn drop(&mut self) {
        debug!("Connection {} is getting closed", self.connection_id);
        // if we never managed to start a proxy, the entry will not exist in the controller
        if self.started_proxy {
            self.controller_sender
                .unbounded_send(ControllerCommand::Remove(self.connection_id))
                .unwrap();
        }
    }
}

impl SocksClient {
    /// Create a new SOCKClient
    pub fn new(
        stream: TcpStream,
        authenticator: Authenticator,
        input_sender: InputMessageSender,
        service_provider: Recipient,
        controller_sender: ControllerSender,
        self_address: Recipient,
    ) -> Self {
        let connection_id = Self::generate_random();
        SocksClient {
            controller_sender,
            connection_id,
            stream: StreamState::Available(stream),
            auth_nmethods: 0,
            socks_version: 0,
            authenticator,
            input_sender,
            service_provider,
            self_address,
            started_proxy: false,
        }
    }

    fn generate_random() -> ConnectionId {
        let mut rng = rand::rngs::OsRng;
        rng.next_u64()
    }

    // Send an error back to the client
    pub async fn error(&mut self, r: ResponseCode) -> Result<(), SocksProxyError> {
        self.stream.write_all(&[5, r as u8]).await?;
        Ok(())
    }

    /// Shutdown the TcpStream to the client and end the session
    pub async fn shutdown(&mut self) -> Result<(), SocksProxyError> {
        info!("client is shutting down its TCP stream");
        self.stream.shutdown().await?;
        Ok(())
    }

    /// Initializes the new client, checking that the correct Socks version (5)
    /// is in use and that the client is authenticated, then runs the request.
    pub async fn run(&mut self) -> Result<(), SocksProxyError> {
        debug!("New connection from: {}", self.stream.peer_addr()?.ip());
        let mut header = [0u8; 2];
        // Read a byte from the stream and determine the version being requested
        self.stream.read_exact(&mut header).await?;

        self.socks_version = header[0];
        self.auth_nmethods = header[1];

        // Handle SOCKS4 requests
        if header[0] != SOCKS_VERSION {
            warn!("Init: Unsupported version: SOCKS{}", self.socks_version);
            self.shutdown().await
        }
        // Valid SOCKS5
        else {
            // Authenticate w/ client
            self.authenticate().await?;
            // Handle requests
            self.handle_request().await
        }
    }

    async fn send_connect_to_mixnet(&mut self, remote_address: RemoteAddress) {
        let req = Request::new_connect(self.connection_id, remote_address, self.self_address);
        let msg = Message::Request(req);

        let input_message = InputMessage::new_fresh(self.service_provider, msg.into_bytes(), false);
        self.input_sender.unbounded_send(input_message).unwrap();
    }

    async fn run_proxy(&mut self, conn_receiver: ConnectionReceiver, remote_proxy_target: String) {
        self.send_connect_to_mixnet(remote_proxy_target.clone())
            .await;

        let stream = self.stream.run_proxy();
        let local_stream_remote = stream
            .peer_addr()
            .expect("failed to extract peer address")
            .to_string();
        let connection_id = self.connection_id;
        let input_sender = self.input_sender.clone();

        let recipient = self.service_provider;
        let (stream, _) = ProxyRunner::new(
            stream,
            local_stream_remote,
            remote_proxy_target,
            conn_receiver,
            input_sender,
            connection_id,
        )
        .run(move |conn_id, read_data, socket_closed| {
            let provider_request = Request::new_send(conn_id, read_data, socket_closed);
            let provider_message = Message::Request(provider_request);
            InputMessage::new_fresh(recipient, provider_message.into_bytes(), false)
        })
        .await
        .into_inner();
        // recover stream from the proxy
        self.stream.finish_proxy(stream)
    }

    /// Handles a client request.
    async fn handle_request(&mut self) -> Result<(), SocksProxyError> {
        debug!("Handling CONNECT Command");

        let request = SocksRequest::from_stream(&mut self.stream).await?;
        let remote_address = request.to_string();

        // setup for receiving from the mixnet
        let (mix_sender, mix_receiver) = mpsc::unbounded();

        match request.command {
            // Use the Proxy to connect to the specified addr/port
            SocksCommand::Connect => {
                trace!("Connecting to: {:?}", remote_address.clone());
                self.acknowledge_socks5().await;

                self.started_proxy = true;
                self.controller_sender
                    .unbounded_send(ControllerCommand::Insert(self.connection_id, mix_sender))
                    .unwrap();

                info!(
                    "Starting proxy for {} (id: {})",
                    remote_address.clone(),
                    self.connection_id
                );
                self.run_proxy(mix_receiver, remote_address.clone()).await;
                info!(
                    "Proxy for {} is finished (id: {})",
                    remote_address, self.connection_id
                );
            }

            SocksCommand::Bind => unimplemented!(), // not handled
            SocksCommand::UdpAssociate => unimplemented!(), // not handled
        };

        Ok(())
    }

    /// Writes a Socks5 header back to the requesting client's TCP stream,
    /// basically saying "I acknowledge your request and am dealing with it".
    async fn acknowledge_socks5(&mut self) {
        self.stream
            .write_all(&[
                SOCKS_VERSION,
                ResponseCode::Success as u8,
                RESERVED,
                1,
                127,
                0,
                0,
                1,
                0,
                0,
            ])
            .await
            .unwrap();
    }

    /// Authenticate the incoming request. Each request is checked for its
    /// authentication method. A user/password request will extract the
    /// username and password from the stream, then check with the Authenticator
    /// to see if the resulting user is allowed.
    ///
    /// A lot of this could probably be put into the `SocksRequest::from_stream()`
    /// constructor, and/or cleaned up with tokio::codec. It's mostly just
    /// read-a-byte-or-two. The bytes being extracted look like this:
    ///
    /// +----+------+----------+------+------------+
    /// |ver | ulen |  uname   | plen |  password  |
    /// +----+------+----------+------+------------+
    /// | 1  |  1   | 1 to 255 |  1   | 1 to 255   |
    /// +----+------+----------+------+------------+
    ///
    /// Pulling out the stream code into its own home, and moving the if/else logic
    /// into the Authenticator (where it'll be more easily testable)
    /// would be a good next step.
    async fn authenticate(&mut self) -> Result<(), SocksProxyError> {
        debug!("Authenticating w/ {}", self.stream.peer_addr()?.ip());
        // Get valid auth methods
        let methods = self.get_available_methods().await?;
        trace!("methods: {:?}", methods);

        let mut response = [0u8; 2];

        // Set the version in the response
        response[0] = SOCKS_VERSION;
        if methods.contains(&(AuthenticationMethods::UserPass as u8)) {
            // Set the default auth method (NO AUTH)
            response[1] = AuthenticationMethods::UserPass as u8;

            debug!("Sending USER/PASS packet");
            self.stream.write_all(&response).await?;

            let mut header = [0u8; 2];

            // Read a byte from the stream and determine the version being requested
            self.stream.read_exact(&mut header).await?;

            // debug!("Auth Header: [{}, {}]", header[0], header[1]);

            // Username parsing
            let ulen = header[1];
            let mut username = vec![0; ulen as usize];
            self.stream.read_exact(&mut username).await?;

            // Password Parsing
            let plen = self.stream.read_u8().await?;
            let mut password = vec![0; plen as usize];
            self.stream.read_exact(&mut password).await?;

            let username_str = String::from_utf8(username)?;
            let password_str = String::from_utf8(password)?;

            let user = User {
                username: username_str,
                password: password_str,
            };

            // Authenticate passwords
            if self.authenticator.is_allowed(&user) {
                debug!("Access Granted. User: {}", user.username);
                let response = [1, ResponseCode::Success as u8];
                self.stream.write_all(&response).await?;
            } else {
                debug!("Access Denied. User: {}", user.username);
                let response = [1, ResponseCode::Failure as u8];
                self.stream.write_all(&response).await?;

                // Shutdown
                self.shutdown().await?;
            }

            Ok(())
        } else if methods.contains(&(AuthenticationMethods::NoAuth as u8)) {
            // set the default auth method (no auth)
            response[1] = AuthenticationMethods::NoAuth as u8;
            debug!("Sending NOAUTH packet");
            self.stream.write_all(&response).await?;
            Ok(())
        } else {
            warn!("Client has no suitable authentication methods!");
            response[1] = AuthenticationMethods::NoMethods as u8;
            self.stream.write_all(&response).await?;
            self.shutdown().await?;
            Err(ResponseCode::Failure.into())
        }
    }

    /// Return the available methods based on `self.auth_nmethods`
    async fn get_available_methods(&mut self) -> Result<Vec<u8>, SocksProxyError> {
        let mut methods: Vec<u8> = Vec::with_capacity(self.auth_nmethods as usize);
        for _ in 0..self.auth_nmethods {
            let mut method = [0u8; 1];
            self.stream.read_exact(&mut method).await?;
            if self.authenticator.auth_methods.contains(&method[0]) {
                methods.append(&mut method.to_vec());
            }
        }
        Ok(methods)
    }
}
