// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::channel::mpsc;
use futures::StreamExt;
use gateway_client::GatewayClient;
use log::*;
use nymsphinx::forwarding::packet::MixPacket;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub type BatchMixMessageSender = mpsc::UnboundedSender<Vec<MixPacket>>;
pub type BatchMixMessageReceiver = mpsc::UnboundedReceiver<Vec<MixPacket>>;

const MAX_FAILURE_COUNT: usize = 100;

pub struct MixTrafficController {
    // TODO: most likely to be replaced by some higher level construct as
    // later on gateway_client will need to be accessible by other entities
    gateway_client: GatewayClient,
    mix_rx: BatchMixMessageReceiver,

    // TODO: this is temporary work-around.
    // in long run `gateway_client` will be moved away from `MixTrafficController` anyway.
    consecutive_gateway_failure_count: usize,
}

impl MixTrafficController {
    pub fn new(
        mix_rx: BatchMixMessageReceiver,
        gateway_client: GatewayClient,
    ) -> MixTrafficController {
        MixTrafficController {
            gateway_client,
            mix_rx,
            consecutive_gateway_failure_count: 0,
        }
    }

    async fn on_messages(&mut self, mut mix_packets: Vec<MixPacket>) {
        debug_assert!(!mix_packets.is_empty());

        let success = if mix_packets.len() == 1 {
            let mix_packet = mix_packets.pop().unwrap();
            self.gateway_client.send_mix_packet(mix_packet).await
        } else {
            self.gateway_client
                .batch_send_mix_packets(mix_packets)
                .await
        };

        match success {
            Err(e) => {
                error!("Failed to send sphinx packet(s) to the gateway! - {:?}", e);
                self.consecutive_gateway_failure_count += 1;
                if self.consecutive_gateway_failure_count == MAX_FAILURE_COUNT {
                    // todo: in the future this should initiate a 'graceful' shutdown or try
                    // to reconnect?
                    panic!("failed to send sphinx packet to the gateway {} times in a row - assuming the gateway is dead. Can't do anything about it yet :(", MAX_FAILURE_COUNT)
                }
            }
            Ok(_) => {
                trace!("We *might* have managed to forward sphinx packet(s) to the gateway!");
                self.consecutive_gateway_failure_count = 0;
            }
        }
    }

    pub async fn run(&mut self) {
        while let Some(mix_packets) = self.mix_rx.next().await {
            self.on_messages(mix_packets).await;
        }
    }

    pub fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            self.run().await;
        })
    }
}
