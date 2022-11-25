use crate::error::Socks5ClientError;

use super::{
    authentication::Authenticator,
    client::SocksClient,
    mixnet_responses::MixnetResponseListener,
    types::{ResponseCodeV4, ResponseCodeV5},
    SocksVersion,
};
use client_connections::{ConnectionCommandSender, LaneQueueLengths};
use client_core::client::{
    inbound_messages::InputMessageSender, received_buffer::ReceivedBufferRequestSender,
};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use proxy_helpers::connection_controller::{BroadcastActiveConnections, Controller};
use std::net::SocketAddr;
use tap::TapFallible;
use task::ShutdownListener;
use tokio::net::TcpListener;

/// A Socks5 server that listens for connections.
pub struct SphinxSocksServer {
    authenticator: Authenticator,
    listening_address: SocketAddr,
    service_provider: Recipient,
    self_address: Recipient,
    lane_queue_lengths: LaneQueueLengths,
    shutdown: ShutdownListener,
}

impl SphinxSocksServer {
    /// Create a new SphinxSocks instance
    pub(crate) fn new(
        port: u16,
        authenticator: Authenticator,
        service_provider: Recipient,
        self_address: Recipient,
        lane_queue_lengths: LaneQueueLengths,
        shutdown: ShutdownListener,
    ) -> Self {
        // hardcode ip as we (presumably) ONLY want to listen locally. If we change it, we can
        // just modify the config
        let ip = "127.0.0.1";
        info!("Listening on {}:{}", ip, port);
        SphinxSocksServer {
            authenticator,
            listening_address: format!("{}:{}", ip, port).parse().unwrap(),
            service_provider,
            self_address,
            lane_queue_lengths,
            shutdown,
        }
    }

    /// Set up the listener and initiate connection handling when something
    /// connects to the server.
    pub(crate) async fn serve(
        &mut self,
        input_sender: InputMessageSender,
        buffer_requester: ReceivedBufferRequestSender,
        client_connection_tx: ConnectionCommandSender,
    ) -> Result<(), Socks5ClientError> {
        let listener = TcpListener::bind(self.listening_address)
            .await
            .tap_err(|err| log::error!("Failed to bind to address: {err}"))?;
        info!("Serving Connections...");

        // controller for managing all active connections
        let (mut active_streams_controller, controller_sender) = Controller::new(
            client_connection_tx,
            BroadcastActiveConnections::Off,
            self.shutdown.clone(),
        );
        tokio::spawn(async move {
            active_streams_controller.run().await;
        });

        // listener for mix messages
        let mut mixnet_response_listener = MixnetResponseListener::new(
            buffer_requester,
            controller_sender.clone(),
            self.shutdown.clone(),
        );
        tokio::spawn(async move {
            mixnet_response_listener.run().await;
        });

        loop {
            tokio::select! {
                Ok((stream, _remote)) = listener.accept() => {
                    // TODO Optimize this
                    let mut client = SocksClient::new(
                        stream,
                        self.authenticator.clone(),
                        input_sender.clone(),
                        &self.service_provider,
                        controller_sender.clone(),
                        &self.self_address,
                        self.lane_queue_lengths.clone(),
                        self.shutdown.clone(),
                    );

                    tokio::spawn(async move {
                        {
                            if let Err(err) = client.run().await {
                                error!("Error! {}", err);
                                let error_text = format!("{}", err);

                                if client.get_version() == Some(&SocksVersion::V4) {
                                    let response = ResponseCodeV4::RequestRejected;
                                    if client.send_error_v4(response).await.is_err() {
                                        warn!("Failed to send error code");
                                    };
                                } else if client.get_version() == Some(&SocksVersion::V5) {
                                    let response = if error_text.contains("Host") {
                                        ResponseCodeV5::HostUnreachable
                                    } else if error_text.contains("Network") {
                                        ResponseCodeV5::NetworkUnreachable
                                    } else if error_text.contains("ttl") {
                                        ResponseCodeV5::TtlExpired
                                    } else {
                                        ResponseCodeV5::Failure
                                    };

                                    if client.send_error_v5(response).await.is_err() {
                                        warn!("Failed to send error code");
                                    };
                                }
                                if client.shutdown().await.is_err() {
                                    warn!("Failed to shutdown TcpStream");
                                };
                            };
                            // client gets dropped here
                        }
                    });
                },
                _ = self.shutdown.recv() => {
                    log::trace!("SphinxSocksServer: Received shutdown");
                    log::debug!("SphinxSocksServer: Exiting");
                    return Ok(());
                }
            }
        }
    }
}
