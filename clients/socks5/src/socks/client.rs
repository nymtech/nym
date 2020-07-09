#![forbid(unsafe_code)]

use super::authentication::{AuthenticationMethods, User};
use super::request::{SocksCommand, SocksRequest};
use super::{AddrType, ResponseCode, SocksProxyError, RESERVED, SOCKS_VERSION};
use crate::client::inbound_messages::InputMessage;
use crate::client::inbound_messages::InputMessageSender;
use crate::socks::utils;
use log::*;
use nymsphinx::{addressing::clients::Recipient, DestinationAddressBytes, NodeAddressBytes};
use std::net::Shutdown;
use tokio::prelude::*;
use tokio::{self, net::TcpStream};

pub(crate) struct SOCKClient {
    stream: TcpStream,
    auth_nmethods: u8,
    auth_methods: Vec<u8>,
    authenticated_users: Vec<User>,
    socks_version: u8,
    input_sender: InputMessageSender,
}

impl SOCKClient {
    /// Create a new SOCKClient
    pub fn new(
        stream: TcpStream,
        authenticated_users: Vec<User>,
        auth_methods: Vec<u8>,
        input_sender: InputMessageSender,
    ) -> Self {
        SOCKClient {
            stream,
            auth_nmethods: 0,
            socks_version: 0,
            authenticated_users,
            auth_methods,
            input_sender,
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

        trace!(
            "Version: {} Auth nmethods: {}",
            self.socks_version,
            self.auth_nmethods
        );

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
            self.handle_client().await?;
        }

        Ok(())
    }

    /// Handles a client
    pub async fn handle_client(&mut self) -> Result<(), SocksProxyError> {
        debug!("Handling requests for {}", self.stream.peer_addr()?.ip());
        let req = SocksRequest::from_stream(&mut self.stream).await?;

        if req.addr_type == AddrType::V6 {}

        // Log Request
        let displayed_addr = utils::pretty_print_addr(&req.addr_type, &req.addr);
        info!(
            "New Request: Source: {}, Command: {:?} Addr: {}, Port: {}",
            self.stream.peer_addr()?.ip(),
            req.command,
            displayed_addr,
            req.port
        );

        let recipient = Recipient::new(
            // client address
            DestinationAddressBytes::try_from_base58_string(
                "6ho9un9BMqUcfnkRNxQiRodo6ShdJVkqj5ShuPGyydDf",
            )
            .unwrap(),
            // gateway address
            NodeAddressBytes::try_from_base58_string(
                "GYCqU48ndXke9o2434i7zEGv1sWg1cNVswWJfRnY1VTB",
            )
            .unwrap(),
        );

        // Respond
        match req.command {
            // Use the Proxy to connect to the specified addr/port
            SocksCommand::Connect => {
                debug!("Handling CONNECT Command");

                let sock_addr = utils::addr_to_socket(&req.addr_type, &req.addr, req.port)?;
                trace!("Connecting to: {:?}", sock_addr);

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

                let remote_address = format!("{}:{}", displayed_addr, req.port);
                let remote_bytes = remote_address.into_bytes();
                let remote_bytes_len = remote_bytes.len() as u16;
                let foo = remote_bytes_len.to_be_bytes(); // this is [u8; 2];
                let mut buf = foo
                    .iter()
                    .cloned()
                    .chain(remote_bytes.into_iter())
                    .collect::<Vec<_>>();

                self.stream.read_to_end(&mut buf).await.unwrap();
                println!("read: {:?}", buf);

                let input_message = InputMessage::new(recipient, buf);
                self.input_sender.unbounded_send(input_message).unwrap();
            }
            _ => unreachable!("don't want to go there"),
        }

        Ok(())
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
            if self.authenticated(&user) {
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

    /// Check if username + password pair are valid
    fn authenticated(&self, user: &User) -> bool {
        self.authenticated_users.contains(user)
    }

    /// Return the available methods based on `self.auth_nmethods`
    async fn get_available_methods(&mut self) -> Result<Vec<u8>, SocksProxyError> {
        let mut methods: Vec<u8> = Vec::with_capacity(self.auth_nmethods as usize);
        for _ in 0..self.auth_nmethods {
            let mut method = [0u8; 1];
            self.stream.read_exact(&mut method).await?;
            if self.auth_methods.contains(&method[0]) {
                methods.append(&mut method.to_vec());
            }
        }
        Ok(methods)
    }
}
