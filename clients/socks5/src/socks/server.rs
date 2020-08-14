use super::authentication::Authenticator;
use super::client::SocksClient;
use super::{
    mixnet_responses::MixnetResponseListener,
    types::{ResponseCode, SocksProxyError},
};
use client_core::client::{
    inbound_messages::InputMessageSender, received_buffer::ReceivedBufferRequestSender,
};
use futures::lock::Mutex;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

/// A Socks5 server that listens for connections.
pub struct SphinxSocksServer {
    authenticator: Authenticator,
    listening_address: SocketAddr,
    service_provider: Recipient,
}

impl SphinxSocksServer {
    /// Create a new SphinxSocks instance
    pub(crate) fn new(
        port: u16,
        ip: &str,
        authenticator: Authenticator,
        service_provider: Recipient,
    ) -> Self {
        info!("Listening on {}:{}", ip, port);
        SphinxSocksServer {
            authenticator,
            listening_address: format!("{}:{}", ip, port).parse().unwrap(),
            service_provider,
        }
    }

    /// Set up the listener and initiate connection handling when something
    /// connects to the server.
    pub(crate) async fn serve(
        &mut self,
        input_sender: InputMessageSender,
        buffer_requester: ReceivedBufferRequestSender,
    ) -> Result<(), SocksProxyError> {
        info!("Serving Connections...");
        let mut listener = TcpListener::bind(self.listening_address).await.unwrap();

        // todo: probably just create a wrapper type for this guy
        let active_streams = Arc::new(Mutex::new(HashMap::new()));

        let mut mixnet_response_listener =
            MixnetResponseListener::new(buffer_requester, Arc::clone(&active_streams));

        tokio::spawn(async move {
            mixnet_response_listener.run().await;
        });

        loop {
            if let Ok((stream, _remote)) = listener.accept().await {
                // TODO Optimize this
                let mut client = SocksClient::new(
                    stream,
                    self.authenticator.clone(),
                    input_sender.clone(),
                    self.service_provider.clone(),
                    Arc::clone(&active_streams),
                );

                tokio::spawn(async move {
                    {
                        match client.run().await {
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
                        // client gets dropped here
                    }
                });
            }
        }
    }
}
