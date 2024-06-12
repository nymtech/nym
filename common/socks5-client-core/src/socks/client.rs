#![forbid(unsafe_code)]

use super::authentication::{AuthenticationMethods, Authenticator, User};
use super::request::{SocksCommand, SocksRequest};
use super::types::{ResponseCodeV4, ResponseCodeV5, SocksProxyError};
use super::{SocksVersion, RESERVED, SOCKS4_VERSION, SOCKS5_VERSION};
use crate::config;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::SinkExt;
use log::*;
use nym_client_core::client::inbound_messages::{InputMessage, InputMessageSender};
use nym_service_providers_common::interface::{ProviderInterfaceVersion, RequestVersion};
use nym_socks5_proxy_helpers::connection_controller::{
    ConnectionReceiver, ControllerCommand, ControllerSender,
};
use nym_socks5_proxy_helpers::proxy_runner::ProxyRunner;
use nym_socks5_requests::{
    ConnectionId, RemoteAddress, Socks5ProtocolVersion, Socks5ProviderRequest, Socks5Request,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::params::PacketSize;
use nym_sphinx::params::PacketType;
use nym_task::connections::{LaneQueueLengths, TransmissionLane};
use nym_task::TaskClient;
use pin_project::pin_project;
use rand::RngCore;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

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

#[derive(Debug, Copy, Clone)]
pub(crate) struct Config {
    biggest_packet_size: PacketSize,
    provider_interface_version: ProviderInterfaceVersion,
    socks5_protocol_version: Socks5ProtocolVersion,
    use_surbs_for_responses: bool,
    connection_start_surbs: u32,
    per_request_surbs: u32,
}

impl Config {
    pub(crate) fn new(
        biggest_packet_size: PacketSize,
        provider_interface_version: ProviderInterfaceVersion,
        socks5_protocol_version: Socks5ProtocolVersion,
        use_surbs_for_responses: bool,
        debug_config: config::Socks5Debug,
    ) -> Self {
        Self {
            biggest_packet_size,
            provider_interface_version,
            socks5_protocol_version,
            use_surbs_for_responses,
            connection_start_surbs: debug_config.connection_start_surbs,
            per_request_surbs: debug_config.per_request_surbs,
        }
    }

    fn request_version(&self) -> RequestVersion<Socks5Request> {
        RequestVersion {
            provider_interface: self.provider_interface_version,
            provider_protocol: self.socks5_protocol_version,
        }
    }
}

/// A client connecting to the Socks proxy server, because
/// it wants to make a Nym-protected outbound request. Typically, this is
/// something like e.g. a wallet app running on your laptop connecting to
/// `SphinxSocksServer`.
pub(crate) struct SocksClient {
    config: Config,
    controller_sender: ControllerSender,
    stream: StreamState,
    auth_nmethods: u8,
    authenticator: Authenticator,
    socks_version: Option<SocksVersion>,
    input_sender: InputMessageSender,
    connection_id: ConnectionId,
    service_provider: Recipient,
    self_address: Recipient,
    started_proxy: bool,
    lane_queue_lengths: LaneQueueLengths,
    shutdown_listener: TaskClient,
    packet_type: Option<PacketType>,
}

impl Drop for SocksClient {
    fn drop(&mut self) {
        debug!("Connection {} is getting closed", self.connection_id);
        // if we never managed to start a proxy, the entry will not exist in the controller
        if self.started_proxy {
            self.controller_sender
                .unbounded_send(ControllerCommand::Remove {
                    connection_id: self.connection_id,
                })
                .unwrap();
        }
    }
}

impl SocksClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Config,
        stream: TcpStream,
        authenticator: Authenticator,
        input_sender: InputMessageSender,
        service_provider: &Recipient,
        controller_sender: ControllerSender,
        self_address: &Recipient,
        lane_queue_lengths: LaneQueueLengths,
        mut shutdown_listener: TaskClient,
        packet_type: Option<PacketType>,
    ) -> Self {
        // If this task fails and exits, we don't want to send shutdown signal
        shutdown_listener.mark_as_success();

        let connection_id = Self::generate_random();

        SocksClient {
            config,
            controller_sender,
            connection_id,
            stream: StreamState::Available(stream),
            auth_nmethods: 0,
            socks_version: None,
            authenticator,
            input_sender,
            service_provider: *service_provider,
            self_address: *self_address,
            started_proxy: false,
            lane_queue_lengths,
            shutdown_listener,
            packet_type,
        }
    }

    fn generate_random() -> ConnectionId {
        let mut rng = rand::rngs::OsRng;
        rng.next_u64()
    }

    pub async fn send_error(&mut self, err: SocksProxyError) -> Result<(), SocksProxyError> {
        let error_text = format!("{err}");
        let Some(ref version) = self.socks_version else {
            log::error!("Trying to send error without knowing the version");
            return Ok(());
        };

        match version {
            SocksVersion::V4 => {
                let response = ResponseCodeV4::RequestRejected;
                self.send_error_v4(response).await
            }
            SocksVersion::V5 => {
                let response = if error_text.contains("Host") {
                    ResponseCodeV5::HostUnreachable
                } else if error_text.contains("Network") {
                    ResponseCodeV5::NetworkUnreachable
                } else if error_text.contains("ttl") {
                    ResponseCodeV5::TtlExpired
                } else {
                    ResponseCodeV5::Failure
                };
                self.send_error_v5(response).await
            }
        }
    }

    // Send an error back to the client
    pub async fn send_error_v4(&mut self, r: ResponseCodeV4) -> Result<(), SocksProxyError> {
        self.stream
            .write_all(&[SOCKS4_VERSION, r as u8])
            .await
            .map_err(|source| SocksProxyError::SocketWriteError { source })
    }

    pub async fn send_error_v5(&mut self, r: ResponseCodeV5) -> Result<(), SocksProxyError> {
        self.stream
            .write_all(&[SOCKS5_VERSION, r as u8])
            .await
            .map_err(|source| SocksProxyError::SocketWriteError { source })
    }

    /// Shutdown the `TcpStream` to the client and end the session
    pub async fn shutdown(&mut self) -> Result<(), SocksProxyError> {
        info!("client is shutting down its TCP stream");
        self.stream
            .shutdown()
            .await
            .map_err(|source| SocksProxyError::SocketShutdownFailure { source })?;
        self.shutdown_listener.mark_as_success();
        Ok(())
    }

    /// Initializes the new client, checking that the correct Socks version (5)
    /// is in use and that the client is authenticated, then runs the request.
    pub async fn run(&mut self) -> Result<(), SocksProxyError> {
        debug!(
            "New connection from: {}",
            self.stream
                .peer_addr()
                .map_err(|source| SocksProxyError::PeerAddrExtractionFailure { source })?
                .ip()
        );

        // Read a byte from the stream and determine the version being requested
        let mut header = [0u8];
        self.stream
            .read_exact(&mut header)
            .await
            .map_err(|source| SocksProxyError::SocketReadError { source })?;

        self.socks_version = match SocksVersion::try_from(header[0]) {
            Ok(version) => Some(version),
            Err(_err) => {
                warn!("Init: Unsupported version: SOCKS{}", header[0]);
                return self.shutdown().await;
            }
        };

        if self.socks_version == Some(SocksVersion::V5) {
            let mut auth = [0u8];
            self.stream
                .read_exact(&mut auth)
                .await
                .map_err(|source| SocksProxyError::SocketReadError { source })?;
            self.auth_nmethods = auth[0];
            self.authenticate_socks5().await?;
        }

        self.handle_request().await
    }

    async fn send_anonymous_connect_to_mixnet(&mut self, remote_address: RemoteAddress) {
        // TODO: simplify by using `request_version`
        let req = Socks5Request::new_connect(
            self.config.socks5_protocol_version,
            self.connection_id,
            remote_address,
            None,
        );
        let msg =
            Socks5ProviderRequest::new_provider_data(self.config.provider_interface_version, req);

        let input_message = InputMessage::new_anonymous(
            self.service_provider,
            msg.into_bytes(),
            self.config.connection_start_surbs,
            TransmissionLane::ConnectionId(self.connection_id),
            self.packet_type,
        );
        self.input_sender
            .send(input_message)
            .await
            .expect("InputMessageReceiver has stopped receiving!");
    }

    async fn send_connect_to_mixnet_with_return_address(&mut self, remote_address: RemoteAddress) {
        // TODO: simplify by using `request_version`
        let req = Socks5Request::new_connect(
            self.config.socks5_protocol_version,
            self.connection_id,
            remote_address,
            Some(self.self_address),
        );
        let msg =
            Socks5ProviderRequest::new_provider_data(self.config.provider_interface_version, req);

        let input_message = InputMessage::new_regular(
            self.service_provider,
            msg.into_bytes(),
            TransmissionLane::ConnectionId(self.connection_id),
            self.packet_type,
        );
        self.input_sender
            .send(input_message)
            .await
            .expect("InputMessageReceiver has stopped receiving!");
    }

    async fn send_connect_to_mixnet(&mut self, remote_address: RemoteAddress) {
        if self.config.use_surbs_for_responses {
            self.send_anonymous_connect_to_mixnet(remote_address).await
        } else {
            self.send_connect_to_mixnet_with_return_address(remote_address)
                .await
        }
    }

    async fn run_proxy(&mut self, conn_receiver: ConnectionReceiver, remote_proxy_target: String) {
        self.send_connect_to_mixnet(remote_proxy_target.clone())
            .await;

        let stream = self.stream.run_proxy();
        let peer_addr = match stream.peer_addr() {
            Ok(peer_addr) => peer_addr,
            Err(err) => {
                log::error!("Unable to extract the remote peer address: {err}");
                return;
            }
        };
        let local_stream_remote = peer_addr.to_string();

        let connection_id = self.connection_id;
        let input_sender = self.input_sender.clone();
        let anonymous = self.config.use_surbs_for_responses;
        let per_request_surbs = self.config.per_request_surbs;
        let request_version = self.config.request_version();

        let recipient = self.service_provider;
        let packet_type = self.packet_type;
        let (stream, _) = ProxyRunner::new(
            stream,
            local_stream_remote,
            remote_proxy_target,
            conn_receiver,
            input_sender,
            // FIXME: this does NOT include overhead due to acks or chunking
            // (so actual true plaintext is smaller)
            self.config.biggest_packet_size.plaintext_size(),
            connection_id,
            Some(self.lane_queue_lengths.clone()),
            self.shutdown_listener.clone(),
        )
        .run(move |socket_data| {
            let lane = TransmissionLane::ConnectionId(socket_data.header.connection_id);
            let provider_request =
                Socks5Request::new_send(request_version.provider_protocol, socket_data);
            let provider_message = Socks5ProviderRequest::new_provider_data(
                request_version.provider_interface,
                provider_request,
            );
            if anonymous {
                InputMessage::new_anonymous(
                    recipient,
                    provider_message.into_bytes(),
                    per_request_surbs,
                    lane,
                    packet_type,
                )
            } else {
                InputMessage::new_regular(
                    recipient,
                    provider_message.into_bytes(),
                    lane,
                    packet_type,
                )
            }
        })
        .await
        .into_inner();
        // recover stream from the proxy
        self.stream.finish_proxy(stream)
    }

    /// Handles a client request.
    async fn handle_request(&mut self) -> Result<(), SocksProxyError> {
        debug!("Handling CONNECT Command");

        let version = self
            .socks_version
            .as_ref()
            .expect("Must read version before parsing request");

        let request = match version {
            SocksVersion::V4 => SocksRequest::from_stream_socks4(&mut self.stream).await?,
            SocksVersion::V5 => SocksRequest::from_stream_socks5(&mut self.stream).await?,
        };

        let remote_address = request.address_string();

        // setup for receiving from the mixnet
        let (mix_sender, mix_receiver) = mpsc::unbounded();

        match request.command {
            // Use the Proxy to connect to the specified addr/port
            SocksCommand::Connect => {
                trace!("Connecting to: {:?}", remote_address.clone());
                match version {
                    SocksVersion::V4 => self.acknowledge_socks4().await,
                    SocksVersion::V5 => self.acknowledge_socks5().await,
                }

                self.started_proxy = true;
                self.controller_sender
                    .unbounded_send(ControllerCommand::Insert {
                        connection_id: self.connection_id,
                        connection_sender: mix_sender,
                    })
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

            SocksCommand::Bind => return Err(SocksProxyError::BindNotSupported), // not handled
            SocksCommand::UdpAssociate => return Err(SocksProxyError::UdpNotSupported),
        };

        Ok(())
    }

    /// Writes a Socks5 header back to the requesting client's TCP stream,
    /// basically saying "I acknowledge your request and am dealing with it".
    async fn acknowledge_socks5(&mut self) {
        self.stream
            .write_all(&[
                SOCKS5_VERSION,
                ResponseCodeV5::Success as u8,
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

    /// Writes a Socks4 header back to the requesting client's TCP stream,
    async fn acknowledge_socks4(&mut self) {
        self.stream
            .write_all(&[
                0, //SOCKS4_VERSION,
                ResponseCodeV4::Granted as u8,
                0,
                0,
                127,
                0,
                0,
                1,
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
    /// constructor, and/or cleaned up with `tokio::codec`. It's mostly just
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
    async fn authenticate_socks5(&mut self) -> Result<(), SocksProxyError> {
        debug!(
            "Authenticating w/ {}",
            self.stream
                .peer_addr()
                .map_err(|source| SocksProxyError::PeerAddrExtractionFailure { source })?
                .ip()
        );
        // Get valid auth methods
        let methods = self.get_available_methods().await?;
        trace!("methods: {:?}", methods);

        let mut response = [0u8; 2];

        // Set the version in the response
        response[0] = SOCKS5_VERSION;
        if methods.contains(&(AuthenticationMethods::UserPass as u8)) {
            // Set the default auth method (NO AUTH)
            response[1] = AuthenticationMethods::UserPass as u8;

            debug!("Sending USER/PASS packet");
            self.stream
                .write_all(&response)
                .await
                .map_err(|source| SocksProxyError::SocketWriteError { source })?;

            let mut header = [0u8; 2];

            // Read a byte from the stream and determine the version being requested
            self.stream
                .read_exact(&mut header)
                .await
                .map_err(|source| SocksProxyError::SocketReadError { source })?;

            // debug!("Auth Header: [{}, {}]", header[0], header[1]);

            // Username parsing
            let ulen = header[1];
            let mut username = vec![0; ulen as usize];
            self.stream
                .read_exact(&mut username)
                .await
                .map_err(|source| SocksProxyError::SocketReadError { source })?;

            // Password Parsing
            let plen = self
                .stream
                .read_u8()
                .await
                .map_err(|source| SocksProxyError::SocketReadError { source })?;
            let mut password = vec![0; plen as usize];
            self.stream
                .read_exact(&mut password)
                .await
                .map_err(|source| SocksProxyError::SocketReadError { source })?;

            let username_str = String::from_utf8(username)
                .map_err(|source| SocksProxyError::MalformedAuthUsername { source })?;
            let password_str = String::from_utf8(password)
                .map_err(|source| SocksProxyError::MalformedAuthPassword { source })?;

            let user = User {
                username: username_str,
                password: password_str,
            };

            // Authenticate passwords
            if self.authenticator.is_allowed(&user) {
                debug!("Access Granted. User: {}", user.username);
                let response = [1, ResponseCodeV5::Success as u8];
                self.stream
                    .write_all(&response)
                    .await
                    .map_err(|source| SocksProxyError::SocketWriteError { source })?;
            } else {
                debug!("Access Denied. User: {}", user.username);
                let response = [1, ResponseCodeV5::Failure as u8];
                self.stream
                    .write_all(&response)
                    .await
                    .map_err(|source| SocksProxyError::SocketWriteError { source })?;

                // Shutdown
                self.shutdown().await?;
            }

            Ok(())
        } else if methods.contains(&(AuthenticationMethods::NoAuth as u8)) {
            // set the default auth method (no auth)
            response[1] = AuthenticationMethods::NoAuth as u8;
            debug!("Sending NOAUTH packet");
            self.stream
                .write_all(&response)
                .await
                .map_err(|source| SocksProxyError::SocketWriteError { source })?;
            Ok(())
        } else {
            warn!("Client has no suitable authentication methods!");
            response[1] = AuthenticationMethods::NoMethods as u8;
            self.stream
                .write_all(&response)
                .await
                .map_err(|source| SocksProxyError::SocketWriteError { source })?;
            self.shutdown().await?;
            Err(ResponseCodeV5::Failure.into())
        }
    }

    /// Return the available methods based on `self.auth_nmethods`
    async fn get_available_methods(&mut self) -> Result<Vec<u8>, SocksProxyError> {
        let mut methods: Vec<u8> = Vec::with_capacity(self.auth_nmethods as usize);
        for _ in 0..self.auth_nmethods {
            let mut method = [0u8; 1];
            self.stream
                .read_exact(&mut method)
                .await
                .map_err(|source| SocksProxyError::SocketReadError { source })?;
            if self.authenticator.auth_methods.contains(&method[0]) {
                methods.append(&mut method.to_vec());
            }
        }
        Ok(methods)
    }
}
