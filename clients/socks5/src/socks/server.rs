use crate::error::Socks5ClientError;

use super::{
    authentication::Authenticator, client::SocksClient, mixnet_responses::MixnetResponseListener,
};
use crate::socks::client;
use client_core::client::{
    inbound_messages::InputMessageSender, received_buffer::ReceivedBufferRequestSender,
};
use log::*;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::{ConnectionCommandSender, LaneQueueLengths};
use nym_task::TaskClient;
use proxy_helpers::connection_controller::Controller;
use std::net::SocketAddr;
use tap::TapFallible;
use tokio::net::TcpListener;

/// A Socks5 server that listens for connections.
pub struct SphinxSocksServer {
    authenticator: Authenticator,
    listening_address: SocketAddr,
    service_provider: Recipient,
    self_address: Recipient,
    client_config: client::Config,
    lane_queue_lengths: LaneQueueLengths,
    shutdown: TaskClient,
}

impl SphinxSocksServer {
    /// Create a new SphinxSocks instance
    pub(crate) fn new(
        port: u16,
        authenticator: Authenticator,
        service_provider: Recipient,
        self_address: Recipient,
        lane_queue_lengths: LaneQueueLengths,
        client_config: client::Config,
        shutdown: TaskClient,
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
            client_config,
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
            //BroadcastActiveConnections::Off,
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

        // TODO:, if required, there should be another task here responsible for control requests.
        // it should get `input_sender` to send actual requests into the mixnet
        // and some channel that connects it from `MixnetResponseListener` to receive
        // any control responses

        loop {
            tokio::select! {
                Ok((stream, _remote)) = listener.accept() => {
                    let mut client = SocksClient::new(
                        self.client_config,
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
                        if let Err(err) = client.run().await {
                            error!("Error! {err}");
                            if client.send_error(err).await.is_err() {
                                warn!("Failed to send error code");
                            };
                            if client.shutdown().await.is_err() {
                                warn!("Failed to shutdown TcpStream");
                            };
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
