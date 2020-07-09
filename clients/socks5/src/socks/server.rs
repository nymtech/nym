use super::authentication::User;
use super::client::SOCKClient;
use super::{ResponseCode, SocksProxyError};
use crate::client::inbound_messages::InputMessageSender;
use log::*;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct SphinxSocks {
    users: Vec<User>,
    auth_methods: Vec<u8>,
    listening_address: SocketAddr,
}

impl SphinxSocks {
    /// Create a new SphinxSocks instance
    pub fn new(port: u16, ip: &str, auth_methods: Vec<u8>, users: Vec<User>) -> Self {
        info!("Listening on {}:{}", ip, port);
        SphinxSocks {
            auth_methods,
            users,
            listening_address: format!("{}:{}", ip, port).parse().unwrap(),
        }
    }

    pub(crate) async fn serve(
        &mut self,
        input_sender: InputMessageSender,
    ) -> Result<(), SocksProxyError> {
        info!("Serving Connections...");
        let mut listener = TcpListener::bind(self.listening_address).await.unwrap();
        loop {
            if let Ok((stream, _remote)) = listener.accept().await {
                // TODO Optimize this
                let mut client = SOCKClient::new(
                    stream,
                    self.users.clone(),
                    self.auth_methods.clone(),
                    input_sender.clone(),
                );

                tokio::spawn(async move {
                    {
                        match client.init().await {
                            Ok(_) => {}
                            Err(error) => {
                                error!("Error! {}", error);
                                let error_text = format!("{}", error);

                                let response: ResponseCode;

                                if error_text.contains("Host") {
                                    response = ResponseCode::HostUnreachable;
                                } else if error_text.contains("Network") {
                                    response = ResponseCode::NetworkUnreachable;
                                } else if error_text.contains("ttl") {
                                    response = ResponseCode::TtlExpired
                                } else {
                                    response = ResponseCode::Failure
                                }

                                if client.error(response).await.is_err() {
                                    warn!("Failed to send error code");
                                };
                                if client.shutdown().is_err() {
                                    warn!("Failed to shutdown TcpStream");
                                };
                            }
                        };
                    }
                });
            }
        }
    }
}
