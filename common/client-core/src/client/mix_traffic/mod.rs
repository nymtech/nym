// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::transceiver::GatewayTransceiver;
use crate::error::ClientCoreError;
use crate::spawn_future;
use log::*;
use nym_sphinx::forwarding::packet::MixPacket;

pub type BatchMixMessageSender = tokio::sync::mpsc::Sender<Vec<MixPacket>>;
pub type BatchMixMessageReceiver = tokio::sync::mpsc::Receiver<Vec<MixPacket>>;

pub mod transceiver;

// We remind ourselves that 32 x 32kb = 1024kb, a reasonable size for a network buffer.
pub const MIX_MESSAGE_RECEIVER_BUFFER_SIZE: usize = 32;
const MAX_FAILURE_COUNT: usize = 100;

// that's also disgusting.
pub struct Empty;

pub struct MixTrafficController {
    gateway_transceiver: Box<dyn GatewayTransceiver + Send>,

    mix_rx: BatchMixMessageReceiver,

    // TODO: this is temporary work-around.
    // in long run `gateway_client` will be moved away from `MixTrafficController` anyway.
    consecutive_gateway_failure_count: usize,
}

impl MixTrafficController {
    pub fn new<T>(gateway_transceiver: T) -> (MixTrafficController, BatchMixMessageSender)
    where
        T: GatewayTransceiver + Send + 'static,
    {
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        (
            MixTrafficController {
                gateway_transceiver: Box::new(gateway_transceiver),
                mix_rx: message_receiver,
                consecutive_gateway_failure_count: 0,
            },
            message_sender,
        )
    }

    pub fn new_dynamic(
        gateway_transceiver: Box<dyn GatewayTransceiver + Send>,
    ) -> (MixTrafficController, BatchMixMessageSender) {
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        (
            MixTrafficController {
                gateway_transceiver,
                mix_rx: message_receiver,
                consecutive_gateway_failure_count: 0,
            },
            message_sender,
        )
    }

    async fn on_messages(
        &mut self,
        mut mix_packets: Vec<MixPacket>,
    ) -> Result<(), ClientCoreError> {
        debug_assert!(!mix_packets.is_empty());
        info!(
            "JON: MixTrafficController: Sending {} sphinx packets to the gateway",
            mix_packets.len()
        );

        let result = if mix_packets.len() == 1 {
            let mix_packet = mix_packets.pop().unwrap();
            self.gateway_transceiver.send_mix_packet(mix_packet).await
        } else {
            self.gateway_transceiver
                .batch_send_mix_packets(mix_packets)
                .await
        };

        let r = match result {
            Err(err) => {
                error!("Failed to send sphinx packet(s) to the gateway: {err}");
                self.consecutive_gateway_failure_count += 1;
                if self.consecutive_gateway_failure_count == MAX_FAILURE_COUNT {
                    // todo: in the future this should initiate a 'graceful' shutdown or try
                    // to reconnect?
                    panic!("failed to send sphinx packet to the gateway {MAX_FAILURE_COUNT} times in a row - assuming the gateway is dead. Can't do anything about it yet :(")
                }
            }
            Ok(_) => {
                trace!("We *might* have managed to forward sphinx packet(s) to the gateway!");
                self.consecutive_gateway_failure_count = 0;
            }
        };
        info!("JON: MixTrafficController: done sending sphinx packets to the gateway");
        Ok(r)
    }

    pub fn start_with_shutdown(mut self, mut shutdown: nym_task::TaskClient) {
        spawn_future(async move {
            debug!("Started MixTrafficController with graceful shutdown support");

            // let mut shutdown0 = shutdown.recv_with_delay();
            // tokio::pin!(shutdown0);

            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.recv_with_delay() => {
                    // _ = &mut shutdown0 => {
                        log::trace!("MixTrafficController: Received shutdown");
                        break;
                    }
                    mix_packets = self.mix_rx.recv() => match mix_packets {
                        Some(mix_packets) => {
                            log::info!("JON: MixTrafficController: mix_rx recv");
                            if let Err(err) = self.on_messages(mix_packets).await {
                                log::error!("MixTrafficController: failed to send mix packets to the gateway: {err}");
                            }
                            // let r = tokio::time::timeout(tokio::time::Duration::from_secs(4), self.on_messages(mix_packets)).await;
                            // if r.is_err() {
                            //     error!("MixTrafficController: Failed to send mix packets to the gateway");
                            // }
                            log::info!("JON: MixTrafficController: done with mix_rx recv");
                        },
                        None => {
                            log::trace!("MixTrafficController: Stopping since channel closed");
                            break;
                        }
                    },
                }
            }
            shutdown.recv_timeout().await;
            log::debug!("MixTrafficController: Exiting");
        })
    }
}
