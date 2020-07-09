#![forbid(unsafe_code)]

use std::net::Shutdown;

use log::*;
use tokio::prelude::*;
use tokio::{self, net::TcpStream};

use nymsphinx::addressing::clients::Recipient;

use crate::client::inbound_messages::InputMessage;
use crate::client::inbound_messages::InputMessageSender;

use super::authentication::{AuthenticationMethods, Authenticator, User};
use super::request::{SocksCommand, SocksRequest};
use super::{ResponseCode, SocksProxyError, RESERVED, SOCKS_VERSION};

pub(crate) struct SocksClient {
    stream: TcpStream,
    auth_nmethods: u8,
    authenticator: Authenticator,
    socks_version: u8,
    input_sender: InputMessageSender,
    service_provider: Recipient,
}

impl SocksClient {
    /// Create a new SOCKClient
    pub fn new(
        stream: TcpStream,
        authenticator: Authenticator,
        input_sender: InputMessageSender,
        service_provider: Recipient,
    ) -> Self {
        SocksClient {
            stream,
            auth_nmethods: 0,
            socks_version: 0,
            authenticator,
            input_sender,
            service_provider,
        }
    }

    // Send an error to the client
    pub async fn error(&mut self, r: ResponseCode) -> Result<(), SocksProxyError> {
        self.stream.write_all(&[5, r as u8]).await?;
        Ok(())
    }

    /// Shutdown a client
    pub fn shutdown(&mut self) -> Result<(), SocksProxyError> {
        self.stream.shutdown(Shutdown::Both)?;
        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), SocksProxyError> {
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

    /// Handles a client
    async fn handle_request(&mut self) -> Result<(), SocksProxyError> {
        debug!("Handling CONNECT Command");

        let req = SocksRequest::from_stream(&mut self.stream).await?;

        match req.command {
            // Use the Proxy to connect to the specified addr/port
            SocksCommand::Connect => {
                trace!("Connecting to: {:?}", req.to_socket());
                self.write_socks5_response().await;
                let buf = self.serialize(req).await;
                self.send_to_mixnet(buf).await;
            }
            _ => unreachable!("don't want to go there"),
        }
        Ok(())
    }

    async fn send_to_mixnet(&self, buf: Vec<u8>) {
        let input_message = InputMessage::new(self.service_provider.clone(), buf);
        self.input_sender.unbounded_send(input_message).unwrap();
    }

    /// Serialize the destination address and port (as a string) concatenated with
    /// the entirety of the request stream. Return it all as a sequence of bytes.
    async fn serialize(&mut self, req: SocksRequest) -> Vec<u8> {
        let remote_address = req.to_string();
        let remote_bytes = remote_address.into_bytes();
        let remote_bytes_len = remote_bytes.len() as u16;
        let temp_bytes = remote_bytes_len.to_be_bytes(); // this is [u8; 2];
        let mut buf = temp_bytes
            .iter()
            .cloned()
            .chain(remote_bytes.into_iter())
            .collect::<Vec<_>>();

        self.stream.read_to_end(&mut buf).await.unwrap(); // appends the rest of the request stream into buf
        buf
    }

    /// Writes a Socks5 header back to the requesting client's TCP stream,
    /// basically saying "I acknowledge your request".
    async fn write_socks5_response(&mut self) {
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
            let username_length = header[1];

            let mut username = Vec::with_capacity(username_length as usize);

            // For some reason the vector needs to actually be full
            for _ in 0..username_length {
                username.push(0);
            }

            self.stream.read_exact(&mut username).await?;

            // Password Parsing
            let mut password_length = [0u8; 1];
            self.stream.read_exact(&mut password_length).await?;

            let mut password = Vec::with_capacity(password_length[0] as usize);

            // For some reason the vector needs to actually be full
            for _ in 0..password_length[0] {
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
