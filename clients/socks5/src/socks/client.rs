#![forbid(unsafe_code)]

use rand::RngCore;
use std::{collections::HashMap, net::Shutdown, sync::Arc};

use log::*;
use tokio::prelude::*;
use tokio::{self, net::TcpStream};

use nymsphinx::addressing::clients::Recipient;

use client_core::client::inbound_messages::InputMessage;
use client_core::client::inbound_messages::InputMessageSender;
use futures::{channel::oneshot, lock::Mutex};

use super::authentication::{AuthenticationMethods, Authenticator, User};
use super::request::{SocksCommand, SocksRequest};
use super::types::{ResponseCode, SocksProxyError};
use super::{RESERVED, SOCKS_VERSION};
use simple_socks5_requests::{ConnectionId, Request};

/// A client connecting to the Socks proxy server, because
/// it wants to make a Nym-protected outbound request. Typically, this is
/// something like e.g. a wallet app running on your laptop connecting to
/// SphinxSocksServer.
pub(crate) struct SocksClient {
    active_streams: ActiveStreams,
    stream: TcpStream,
    auth_nmethods: u8,
    authenticator: Authenticator,
    socks_version: u8,
    input_sender: InputMessageSender,
    connection_id: ConnectionId,
    service_provider: Recipient,
}

type StreamResponseSender = oneshot::Sender<Vec<u8>>;

pub(crate) type ActiveStreams = Arc<Mutex<HashMap<ConnectionId, StreamResponseSender>>>;

impl Drop for SocksClient {
    fn drop(&mut self) {
        println!("socksclient is going out of scope - the stream is getting dropped!")
    }
}

impl SocksClient {
    /// Create a new SOCKClient
    pub fn new(
        stream: TcpStream,
        authenticator: Authenticator,
        input_sender: InputMessageSender,
        service_provider: Recipient,
        active_streams: ActiveStreams,
    ) -> Self {
        let connection_id = Self::generate_random();
        SocksClient {
            active_streams,
            connection_id,
            stream,
            auth_nmethods: 0,
            socks_version: 0,
            authenticator,
            input_sender,
            service_provider,
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
    pub fn shutdown(&mut self) -> Result<(), SocksProxyError> {
        println!("client is shutting down its TCP stream");
        self.stream.shutdown(Shutdown::Both)?;
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
            self.shutdown()?;
        }
        // Valid SOCKS5
        else {
            // Authenticate w/ client
            self.authenticate().await?;
            // Handle requests
            self.handle_request().await?;
        }

        Ok(())
    }

    async fn send_request_to_mixnet(&mut self, request: Request) {
        self.send_to_mixnet(request.into_bytes()).await;
    }

    async fn send_request_to_mixnet_and_get_response(&mut self, request: Request) -> Vec<u8> {
        self.send_to_mixnet(request.into_bytes()).await;

        // refactor idea: crossbeam oneshot channels are faster
        let (sender, receiver) = oneshot::channel();
        let mut active_streams_guard = self.active_streams.lock().await;
        if active_streams_guard
            .insert(self.connection_id, sender)
            .is_some()
        {
            panic!("there is already an active request with the same id present - it's probably a bug!")
        };
        drop(active_streams_guard);
        receiver.await.unwrap()
    }

    /// Handles a client request.
    async fn handle_request(&mut self) -> Result<(), SocksProxyError> {
        debug!("Handling CONNECT Command");

        let request = SocksRequest::from_stream(&mut self.stream).await?;
        let remote_address = request.to_string();

        match request.command {
            // Use the Proxy to connect to the specified addr/port
            SocksCommand::Connect => {
                trace!("Connecting to: {:?}", request.to_socket());
                self.acknowledge_socks5().await;

                let request_data_bytes =
                    SocksRequest::try_read_request_data(&mut self.stream).await?;
                let socks_provider_request = Request::new_connect(
                    self.connection_id,
                    remote_address.clone(),
                    request_data_bytes,
                );
                let response_data = self
                    .send_request_to_mixnet_and_get_response(socks_provider_request)
                    .await;
                self.stream.write_all(&response_data).await.unwrap();

                loop {
                    if let Ok(request_data_bytes) =
                        SocksRequest::try_read_request_data(&mut self.stream).await
                    {
                        if request_data_bytes.is_empty() {
                            break;
                        }
                        let socks_provider_request =
                            Request::new_send(self.connection_id, request_data_bytes);
                        let response_data = self
                            .send_request_to_mixnet_and_get_response(socks_provider_request)
                            .await;
                        self.stream.write_all(&response_data).await.unwrap();
                    } else {
                        break;
                    }
                }
                let socks_provider_request = Request::new_close(self.connection_id);
                self.send_request_to_mixnet(socks_provider_request).await;
                // TODO: where is connection removed from active connection??
            }

            SocksCommand::Bind => unimplemented!(), // not handled
            SocksCommand::UdpAssociate => unimplemented!(), // not handled
        };
        Ok(())
    }

    /// Send serialized Socks5 request bytes to the mixnet. The request stream
    /// will be chunked up into a series of one or more Sphinx packets and
    /// reassembled at the destination service provider at the other end, then
    /// sent onwards anonymously.
    async fn send_to_mixnet(&self, request_bytes: Vec<u8>) {
        let input_message =
            InputMessage::new_fresh(self.service_provider.clone(), request_bytes, false);
        self.input_sender.unbounded_send(input_message).unwrap();
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
    /// A lot of this could probably be put into the the `SocksRequest::from_stream()`
    /// constructor, and/or cleaned up with tokio::codec. It's mostly just
    /// read-a-byte-or-two. The bytes being extracted look like this:
    ///
    /// +----+------+----------+------+------------+
    //  |ver | ulen |  uname   | plen |  password  |
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

            let mut username = Vec::with_capacity(ulen as usize);

            // For some reason the vector needs to actually be full
            for _ in 0..ulen {
                username.push(0);
            }

            self.stream.read_exact(&mut username).await?;

            // Password Parsing
            let mut plen = [0u8; 1];
            self.stream.read_exact(&mut plen).await?;

            let mut password = Vec::with_capacity(plen[0] as usize);

            // For some reason the vector needs to actually be full
            for _ in 0..plen[0] {
                password.push(0);
            }

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
                self.shutdown()?;
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
            self.shutdown()?;
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
