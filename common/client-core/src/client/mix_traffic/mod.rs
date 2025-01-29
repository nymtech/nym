// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::transceiver::GatewayTransceiver;
use crate::error::ClientCoreError;
use crate::spawn_future;
use log::*;
use nym_gateway_requests::ClientRequest;
use nym_sphinx::forwarding::packet::MixPacket;
use transceiver::ErasedGatewayError;

pub type BatchMixMessageSender = tokio::sync::mpsc::Sender<Vec<MixPacket>>;
pub type BatchMixMessageReceiver = tokio::sync::mpsc::Receiver<Vec<MixPacket>>;
pub type ClientRequestReceiver = tokio::sync::mpsc::Receiver<ClientRequest>;
pub type ClientRequestSender = tokio::sync::mpsc::Sender<ClientRequest>;

pub mod transceiver;

// We remind ourselves that 32 x 32kb = 1024kb, a reasonable size for a network buffer.
pub const MIX_MESSAGE_RECEIVER_BUFFER_SIZE: usize = 32;
const MAX_FAILURE_COUNT: usize = 100;

// that's also disgusting.
pub struct Empty;

pub struct MixTrafficController {
    gateway_transceiver: Box<dyn GatewayTransceiver + Send>,

    mix_rx: BatchMixMessageReceiver,
    client_rx: ClientRequestReceiver,

    // TODO: this is temporary work-around.
    // in long run `gateway_client` will be moved away from `MixTrafficController` anyway.
    consecutive_gateway_failure_count: usize,
}

impl MixTrafficController {
    pub fn new<T>(
        gateway_transceiver: T,
    ) -> (
        MixTrafficController,
        BatchMixMessageSender,
        ClientRequestSender,
    )
    where
        T: GatewayTransceiver + Send + 'static,
    {
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);

        let (client_sender, client_receiver) = tokio::sync::mpsc::channel(1);

        (
            MixTrafficController {
                gateway_transceiver: Box::new(gateway_transceiver),
                mix_rx: message_receiver,
                client_rx: client_receiver,
                consecutive_gateway_failure_count: 0,
            },
            message_sender,
            client_sender,
        )
    }

    pub fn new_dynamic(
        gateway_transceiver: Box<dyn GatewayTransceiver + Send>,
    ) -> (
        MixTrafficController,
        BatchMixMessageSender,
        ClientRequestSender,
    ) {
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        let (client_sender, client_receiver) = tokio::sync::mpsc::channel(1);
        (
            MixTrafficController {
                gateway_transceiver,
                mix_rx: message_receiver,
                client_rx: client_receiver,
                consecutive_gateway_failure_count: 0,
            },
            message_sender,
            client_sender,
        )
    }

    async fn on_messages(
        &mut self,
        mut mix_packets: Vec<MixPacket>,
    ) -> Result<(), ErasedGatewayError> {
        debug_assert!(!mix_packets.is_empty());

        let result = if mix_packets.len() == 1 {
            let mix_packet = mix_packets.pop().unwrap();
            self.gateway_transceiver.send_mix_packet(mix_packet).await
        } else {
            self.gateway_transceiver
                .batch_send_mix_packets(mix_packets)
                .await
        };

        if result.is_err() {
            self.consecutive_gateway_failure_count += 1;
        } else {
            trace!("We *might* have managed to forward sphinx packet(s) to the gateway!");
            self.consecutive_gateway_failure_count = 0;
        }

        result
    }

    pub fn start_with_shutdown(mut self, mut shutdown: nym_task::TaskClient) {
        spawn_future(async move {
            debug!("Started MixTrafficController with graceful shutdown support");

            loop {
                tokio::select! {
                    mix_packets = self.mix_rx.recv() => match mix_packets {
                        Some(mix_packets) => {
                            if let Err(err) = self.on_messages(mix_packets).await {
                                error!("Failed to send sphinx packet(s) to the gateway: {err}");
                                if self.consecutive_gateway_failure_count == MAX_FAILURE_COUNT {
                                    // Disconnect from the gateway. If we should try to re-connect
                                    // is handled at the higher layer.
                                    error!("failed to send sphinx packet to the gateway {MAX_FAILURE_COUNT} times in a row - assuming the gateway is dead");
                                    // WIP(JON): do we need to handle the embedded mixnet client
                                    // case separately?
                                    // WIP(JON): can't we use the CancellationToken here instead?
                                    shutdown.send_we_stopped(Box::new(ClientCoreError::UnexpectedExit));
                                    break;
                                }
                            }
                        },
                        None => {
                            log::trace!("MixTrafficController: Stopping since channel closed");
                            break;
                        }
                    },
                    client_request = self.client_rx.recv() => match client_request {
                        Some(client_request) => {
                            match self.gateway_transceiver.send_client_request(client_request).await {
                                Ok(_) => (),
                                Err(e) => error!("Failed to send client request: {}", e),
                            };
                        },
                        None => {
                            log::trace!("MixTrafficController, client request channel closed");
                        }
                    },
                    _ = shutdown.recv_with_delay() => {
                        log::trace!("MixTrafficController: Received shutdown");
                        break;
                    }
                }
            }
            shutdown.recv_timeout().await;

            log::debug!("MixTrafficController: Exiting");
        });
    }
}
