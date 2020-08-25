#![forbid(unsafe_code)]

use super::authentication::{AuthenticationMethods, Authenticator, User};
use super::request::{SocksCommand, SocksRequest};
use super::types::{ResponseCode, SocksProxyError};
use super::{RESERVED, SOCKS_VERSION};
use crate::socks::active_streams_controller::{
    ControllerCommand, ControllerSender, StreamResponseReceiver,
};
use client_core::client::inbound_messages::InputMessage;
use client_core::client::inbound_messages::InputMessageSender;
use futures::channel::mpsc;
use futures::{channel::oneshot, lock::Mutex};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use rand::RngCore;
use simple_socks5_requests::{ConnectionId, Request};
use std::{collections::HashMap, net::Shutdown, sync::Arc};
use tokio::prelude::*;
use tokio::stream::StreamExt;
use tokio::sync::Notify;
use tokio::{self, net::TcpStream};

/// A client connecting to the Socks proxy server, because
/// it wants to make a Nym-protected outbound request. Typically, this is
/// something like e.g. a wallet app running on your laptop connecting to
/// SphinxSocksServer.
pub(crate) struct SocksClient {
    controller_sender: ControllerSender,
    stream: Option<TcpStream>,
    auth_nmethods: u8,
    authenticator: Authenticator,
    socks_version: u8,
    input_sender: InputMessageSender,
    connection_id: ConnectionId,
    service_provider: Recipient,
    self_address: Recipient,
}

impl Drop for SocksClient {
    fn drop(&mut self) {
        println!("socksclient is going out of scope - the stream is getting dropped!");
        self.controller_sender
            .unbounded_send(ControllerCommand::Remove(self.connection_id))
            .unwrap();
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
            stream: Some(stream),
            auth_nmethods: 0,
            socks_version: 0,
            authenticator,
            input_sender,
            service_provider,
            self_address,
        }
    }

    fn generate_random() -> ConnectionId {
        let mut rng = rand::rngs::OsRng;
        rng.next_u64()
    }

    // Send an error back to the client
    pub async fn error(&mut self, r: ResponseCode) -> Result<(), SocksProxyError> {
        self.stream
            .as_mut()
            .unwrap()
            .write_all(&[5, r as u8])
            .await?;
        Ok(())
    }

    /// Shutdown the TcpStream to the client and end the session
    pub fn shutdown(&mut self) -> Result<(), SocksProxyError> {
        println!("client is shutting down its TCP stream");
        TcpStream::shutdown(self.stream.as_mut().unwrap(), Shutdown::Both)?;
        Ok(())
    }

    /// Initializes the new client, checking that the correct Socks version (5)
    /// is in use and that the client is authenticated, then runs the request.
    pub async fn run(&mut self) -> Result<(), SocksProxyError> {
        debug!(
            "New connection from: {}",
            self.stream.as_mut().unwrap().peer_addr()?.ip()
        );
        let mut header = [0u8; 2];
        // Read a byte from the stream and determine the version being requested
        self.stream
            .as_mut()
            .unwrap()
            .read_exact(&mut header)
            .await?;

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

    async fn run_proxy(&mut self, mut mix_receiver: StreamResponseReceiver) {
        let notify_closed = Arc::new(Notify::new());
        let notify_clone = Arc::clone(&notify_closed);

        let stream = self.stream.take().unwrap();
        let connection_id = self.connection_id;
        let client_address = stream.peer_addr().unwrap().to_string();

        let (mut read_half, mut write_half) = stream.into_split();

        let provider_address = self.service_provider.clone();
        let input_sender = self.input_sender.clone();

        // should run until either inbound closes or is notified from outbound
        let inbound_future = async move {
            loop {
                tokio::select! {
                    _ = notify_closed.notified() => {
                        // the remote (service provider) socket is closed, so there's no point
                        // in reading anything more because we won't be able to write to remote anyway!
                        break
                    }
                    // basically copy data from client to mixnet until the socket remains open
                    reading_result = SocksRequest::try_read_request_data(&mut read_half, &client_address) => {
                        let (request_data, timed_out) = match reading_result {
                            Ok(data) => data,
                            Err(err) => {
                                error!("failed to read request from the socket - {}", err);
                                break;
                            }
                        };
                        if request_data.is_empty() && !timed_out {
                            debug!("The socket is closed - won't receive any more data");
                            // no point in reading from mixnet if connection is closed!
                            notify_closed.notify();
                            break;
                        }
                        if request_data.is_empty() {
                            // no point in writing empty request
                            continue;
                        }
                        let socks_provider_request = Request::new_send(connection_id, request_data);
                        Self::send_to_mixnet_with_recipient_on_channel(
                            socks_provider_request.into_bytes(),
                            provider_address.clone(),
                            input_sender.clone(),
                        ).await;
                    }
                }
            }

            read_half
        };

        // should run until notified from mixnet or until local connection is closed
        let outbound_future = async move {
            // keep reading from mixnet until close signal
            loop {
                tokio::select! {
                    _ = notify_clone.notified() => {
                        // no need to read from mixnet as we won't be able to send to socket
                        // anyway
                        break
                    }
                    // if channel closed => done?
                    mix_data = mix_receiver.next() => {
                        let (data, remote_closed) = mix_data.unwrap();
                        if let Err(err) = write_half.write_all(&data).await {
                            // the other half is probably going to blow up too (if not, this task also needs to notify the other one!!)
                            error!("failed to write response back to the socket - {}", err)
                        }
                        if remote_closed {
                            println!("remote got closed - let's write what we received and also close!");
                            notify_clone.notify();
                            break
                        }
                    }
                }
            }
            write_half
        };

        let handle_inbound = tokio::spawn(inbound_future);
        let handle_outbound = tokio::spawn(outbound_future);

        let (write_half, read_half) = futures::future::join(handle_inbound, handle_outbound).await;

        if write_half.is_err() || read_half.is_err() {
            panic!("TODO: some future error?")
        }

        self.stream = Some(write_half.unwrap().reunite(read_half.unwrap()).unwrap());
    }

    /// Handles a client request.
    async fn handle_request(&mut self) -> Result<(), SocksProxyError> {
        debug!("Handling CONNECT Command");

        let request = SocksRequest::from_stream(&mut self.stream.as_mut().unwrap()).await?;
        let remote_address = request.to_string();
        let client_address = self
            .stream
            .as_mut()
            .unwrap()
            .peer_addr()
            .unwrap()
            .to_string();

        // setup for receiving from the mixnet
        let (mix_sender, mix_receiver) = mpsc::unbounded();
        self.controller_sender
            .unbounded_send(ControllerCommand::Insert(self.connection_id, mix_sender))
            .unwrap();

        match request.command {
            // Use the Proxy to connect to the specified addr/port
            SocksCommand::Connect => {
                trace!("Connecting to: {:?}", request.to_socket());
                self.acknowledge_socks5().await;

                // 'connect' needs to be handled manually due to different structure
                let (request_data_bytes, _) = SocksRequest::try_read_request_data(
                    &mut self.stream.as_mut().unwrap(),
                    &client_address,
                )
                .await?;
                let socks_provider_request = Request::new_connect(
                    self.connection_id,
                    remote_address.clone(),
                    request_data_bytes,
                    self.self_address.clone(),
                );

                self.send_request_to_mixnet(socks_provider_request).await;
                self.run_proxy(mix_receiver).await;
                self.send_request_to_mixnet(Request::new_close(self.connection_id))
                    .await;
            }

            SocksCommand::Bind => unimplemented!(), // not handled
            SocksCommand::UdpAssociate => unimplemented!(), // not handled
        };

        Ok(())
    }

    // specialised version of `send_to_mixnet` that does not require `Self`
    async fn send_to_mixnet_with_recipient_on_channel(
        request_bytes: Vec<u8>,
        recipient: Recipient,
        input_sender: InputMessageSender,
    ) {
        let input_message = InputMessage::new_fresh(recipient, request_bytes, false);
        input_sender.unbounded_send(input_message).unwrap();
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
            .as_mut()
            .unwrap()
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
    /// |ver | ulen |  uname   | plen |  password  |
    /// +----+------+----------+------+------------+
    /// | 1  |  1   | 1 to 255 |  1   | 1 to 255   |
    /// +----+------+----------+------+------------+
    ///
    /// Pulling out the stream code into its own home, and moving the if/else logic
    /// into the Authenticator (where it'll be more easily testable)
    /// would be a good next step.
    async fn authenticate(&mut self) -> Result<(), SocksProxyError> {
        debug!(
            "Authenticating w/ {}",
            self.stream.as_mut().unwrap().peer_addr()?.ip()
        );
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
            self.stream.as_mut().unwrap().write_all(&response).await?;

            let mut header = [0u8; 2];

            // Read a byte from the stream and determine the version being requested
            self.stream
                .as_mut()
                .unwrap()
                .read_exact(&mut header)
                .await?;

            // debug!("Auth Header: [{}, {}]", header[0], header[1]);

            // Username parsing
            let ulen = header[1];

            let mut username = Vec::with_capacity(ulen as usize);

            // For some reason the vector needs to actually be full
            for _ in 0..ulen {
                username.push(0);
            }

            self.stream
                .as_mut()
                .unwrap()
                .read_exact(&mut username)
                .await?;

            // Password Parsing
            let mut plen = [0u8; 1];
            self.stream.as_mut().unwrap().read_exact(&mut plen).await?;

            let mut password = Vec::with_capacity(plen[0] as usize);

            // For some reason the vector needs to actually be full
            for _ in 0..plen[0] {
                password.push(0);
            }

            self.stream
                .as_mut()
                .unwrap()
                .read_exact(&mut password)
                .await?;

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
                self.stream.as_mut().unwrap().write_all(&response).await?;
            } else {
                debug!("Access Denied. User: {}", user.username);
                let response = [1, ResponseCode::Failure as u8];
                self.stream.as_mut().unwrap().write_all(&response).await?;

                // Shutdown
                self.shutdown()?;
            }

            Ok(())
        } else if methods.contains(&(AuthenticationMethods::NoAuth as u8)) {
            // set the default auth method (no auth)
            response[1] = AuthenticationMethods::NoAuth as u8;
            debug!("Sending NOAUTH packet");
            self.stream.as_mut().unwrap().write_all(&response).await?;
            Ok(())
        } else {
            warn!("Client has no suitable authentication methods!");
            response[1] = AuthenticationMethods::NoMethods as u8;
            self.stream.as_mut().unwrap().write_all(&response).await?;
            self.shutdown()?;
            Err(ResponseCode::Failure.into())
        }
    }

    /// Return the available methods based on `self.auth_nmethods`
    async fn get_available_methods(&mut self) -> Result<Vec<u8>, SocksProxyError> {
        let mut methods: Vec<u8> = Vec::with_capacity(self.auth_nmethods as usize);
        for _ in 0..self.auth_nmethods {
            let mut method = [0u8; 1];
            self.stream
                .as_mut()
                .unwrap()
                .read_exact(&mut method)
                .await?;
            if self.authenticator.auth_methods.contains(&method[0]) {
                methods.append(&mut method.to_vec());
            }
        }
        Ok(methods)
    }
}
