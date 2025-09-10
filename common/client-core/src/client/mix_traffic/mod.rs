// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::transceiver::GatewayTransceiver;
use nym_gateway_requests::ClientRequest;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::*;
use transceiver::ErasedGatewayError;

#[cfg(target_arch = "wasm32")]
use wasm_utils::console_log;

#[derive(Debug)]
pub struct CountedSender<T>(pub Arc<mpsc::Sender<T>>);

impl<T> Clone for CountedSender<T> {
    fn clone(&self) -> Self {
        let cnt = Arc::strong_count(&self.0);
        console_log!("Sender cloned (was {})", cnt);
        CountedSender(Arc::clone(&self.0))
    }
}
impl<T> Drop for CountedSender<T> {
    fn drop(&mut self) {
        let left = Arc::strong_count(&self.0).saturating_sub(1);
        console_log!("Sender dropped, {} left", left);
    }
}

impl<T> CountedSender<T> {
    pub fn send(
        &self,
        value: T,
    ) -> impl std::future::Future<Output = Result<(), mpsc::error::SendError<T>>> + '_ {
        self.0.send(value)
    }
    pub fn try_send(&self, value: T) -> Result<(), mpsc::error::TrySendError<T>> {
        self.0.try_send(value)
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    pub fn max_capacity(&self) -> usize {
        self.0.max_capacity()
    }
}

pub type BatchMixMessageSender = CountedSender<Vec<MixPacket>>;
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

    shutdown_token: ShutdownToken,
}

impl MixTrafficController {
    pub fn new<T>(
        gateway_transceiver: T,
        shutdown_token: ShutdownToken,
    ) -> (
        MixTrafficController,
        BatchMixMessageSender,
        ClientRequestSender,
    )
    where
        T: GatewayTransceiver + Send + 'static,
    {
        console_log!("MixTrafficController::new called");
        console_log!(
            "MixTrafficController: task_client.is_dummy() = {}",
            task_client.is_dummy()
        );

        let (raw_tx, mix_rx) = mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        let mix_tx = CountedSender(Arc::new(raw_tx));
        let (client_tx, client_rx) = tokio::sync::mpsc::channel(8);

        let controller = MixTrafficController {
            gateway_transceiver: Box::new(gateway_transceiver),
            mix_rx,
            client_rx,
            consecutive_gateway_failure_count: 0,
            task_client,
        };

        (
            MixTrafficController {
                gateway_transceiver: Box::new(gateway_transceiver),
                mix_rx: message_receiver,
                client_rx: client_receiver,
                consecutive_gateway_failure_count: 0,
                shutdown_token,
            },
            message_sender,
            client_sender,
        )
    }

    pub fn new_dynamic(
        gateway_transceiver: Box<dyn GatewayTransceiver + Send>,
        shutdown_token: ShutdownToken,
    ) -> (
        MixTrafficController,
        BatchMixMessageSender,
        ClientRequestSender,
    ) {
        console_log!("MixTrafficController::new_dynamic called");
        console_log!(
            "MixTrafficController::new_dynamic: task_client.is_dummy() = {}",
            task_client.is_dummy()
        );
        let (raw_tx, message_receiver) = mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        let message_sender = CountedSender(Arc::new(raw_tx));
        let (client_sender, client_receiver) = tokio::sync::mpsc::channel(8);
        (
            MixTrafficController {
                gateway_transceiver,
                mix_rx: message_receiver,
                client_rx: client_receiver,
                consecutive_gateway_failure_count: 0,
                shutdown_token,
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
        let send_future = if mix_packets.len() == 1 {
            // SAFETY: we just checked we have one packet
            #[allow(clippy::unwrap_used)]
            let mix_packet = mix_packets.pop().unwrap();
            self.gateway_transceiver.send_mix_packet(mix_packet)
        } else {
            self.gateway_transceiver.batch_send_mix_packets(mix_packets)
        };

        tokio::select! {
            biased;
            _ = self.shutdown_token.cancelled() => {
                trace!("received shutdown while handling messages");
                Ok(())
            }
            result = send_future => {
                if result.is_err() {
                    self.consecutive_gateway_failure_count += 1;
                } else {
                    trace!("We *might* have managed to forward sphinx packet(s) to the gateway!");
                    self.consecutive_gateway_failure_count = 0;
                }

                result
            }
        }
    }

    async fn on_client_request(&mut self, client_request: ClientRequest) {
        tokio::select! {
            biased;
             _ = self.shutdown_token.cancelled() => {
                trace!("received shutdown while handling client request");
            }
            result = self.gateway_transceiver.send_client_request(client_request) => {
                if let Err(err) = result {
                    error!("Failed to send client request: {err}")
                }
            }
        }
    }

    pub async fn run(&mut self) {
        debug!("Started MixTrafficController with graceful shutdown support");
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("MixTrafficController: Received shutdown");
                    break;
                }
                mix_packets = self.mix_rx.recv() => match mix_packets {
                    Some(mix_packets) => {
                        if let Err(err) = self.on_messages(mix_packets).await {
                            error!("Failed to send sphinx packet(s) to the gateway: {err}");
                            if self.consecutive_gateway_failure_count == MAX_FAILURE_COUNT {
                                // Disconnect from the gateway. If we should try to re-connect
                                // is handled at a higher layer.
                                error!("Failed to send sphinx packet to the gateway {MAX_FAILURE_COUNT} times in a row - assuming the gateway is dead");
                                // Do we need to handle the embedded mixnet client case
                                // separately?
                                break;
                            }
                        }
                    },
                    None => {
                        trace!("MixTrafficController: Stopping since channel closed");
                        break;
                    }
                },
                client_request = self.client_rx.recv() => match client_request {
                    Some(client_request) => {
                        self.on_client_request(client_request).await;
                    },
                    None => {
                        trace!("MixTrafficController, client request channel closed");
                    break}
                },
            }
        }
        debug!("MixTrafficController: Exiting");
    }
}
