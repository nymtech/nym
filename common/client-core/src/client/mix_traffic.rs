// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::spawn_future;
use log::*;
use nym_gateway_client::GatewayClient;
use nym_sphinx::forwarding::packet::MixPacket;

use nym_credential_storage::storage::Storage;
#[cfg(not(target_arch = "wasm32"))]
use nym_validator_client::nyxd::traits::DkgQueryClient;

#[cfg(target_arch = "wasm32")]
use nym_bandwidth_controller::wasm_mockups::DkgQueryClient;

pub type BatchMixMessageSender = tokio::sync::mpsc::Sender<Vec<MixPacket>>;
pub type BatchMixMessageReceiver = tokio::sync::mpsc::Receiver<Vec<MixPacket>>;

// We remind ourselves that 32 x 32kb = 1024kb, a reasonable size for a network buffer.
pub const MIX_MESSAGE_RECEIVER_BUFFER_SIZE: usize = 32;
const MAX_FAILURE_COUNT: usize = 100;

pub struct MixTrafficController<C, St: Storage> {
    // TODO: most likely to be replaced by some higher level construct as
    // later on gateway_client will need to be accessible by other entities
    gateway_client: GatewayClient<C, St>,
    mix_rx: BatchMixMessageReceiver,

    // TODO: this is temporary work-around.
    // in long run `gateway_client` will be moved away from `MixTrafficController` anyway.
    consecutive_gateway_failure_count: usize,
}

impl<C, St> MixTrafficController<C, St>
where
    C: DkgQueryClient + Sync + Send + 'static,
    St: Storage + 'static,
{
    pub fn new(
        gateway_client: GatewayClient<C, St>,
    ) -> (MixTrafficController<C, St>, BatchMixMessageSender) {
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        (
            MixTrafficController {
                gateway_client,
                mix_rx: message_receiver,
                consecutive_gateway_failure_count: 0,
            },
            message_sender,
        )
    }

    async fn on_messages(&mut self, mut mix_packets: Vec<MixPacket>) {
        debug_assert!(!mix_packets.is_empty());

        let result = if mix_packets.len() == 1 {
            let mix_packet = mix_packets.pop().unwrap();
            self.gateway_client.send_mix_packet(mix_packet).await
        } else {
            self.gateway_client
                .batch_send_mix_packets(mix_packets)
                .await
        };

        match result {
            Err(err) => {
                error!("Failed to send sphinx packet(s) to the gateway! - {err}");
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
        }
    }

    pub fn start_with_shutdown(mut self, mut shutdown: nym_task::TaskClient) {
        spawn_future(async move {
            debug!("Started MixTrafficController with graceful shutdown support");

            loop {
                tokio::select! {
                    mix_packets = self.mix_rx.recv() => match mix_packets {
                        Some(mix_packets) => {
                            self.on_messages(mix_packets).await;
                        },
                        None => {
                            log::trace!("MixTrafficController: Stopping since channel closed");
                            break;
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
        })
    }
}
