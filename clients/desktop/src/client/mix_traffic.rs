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
use nymsphinx::SphinxPacket;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub(crate) struct MixMessage(SocketAddr, SphinxPacket);
pub(crate) type MixMessageSender = mpsc::UnboundedSender<MixMessage>;
pub(crate) type MixMessageReceiver = mpsc::UnboundedReceiver<MixMessage>;

impl MixMessage {
    pub(crate) fn new(address: SocketAddr, packet: SphinxPacket) -> Self {
        MixMessage(address, packet)
    }
}

pub(crate) struct MixTrafficController<'a> {
    // TODO: most likely to be replaced by some higher level construct as
    // later on gateway_client will need to be accessible by other entities
    gateway_client: GatewayClient<'a, url::Url>,
    mix_rx: MixMessageReceiver,
}

impl<'a> MixTrafficController<'static> {
    pub(crate) fn new(
        mix_rx: MixMessageReceiver,
        gateway_client: GatewayClient<'a, url::Url>,
    ) -> MixTrafficController<'a> {
        MixTrafficController {
            gateway_client,
            mix_rx,
        }
    }

    async fn on_message(&mut self, mix_message: MixMessage) {
        debug!("Got a mix_message for {:?}", mix_message.0);
        match self
            .gateway_client
            .send_sphinx_packet(mix_message.0, mix_message.1.to_bytes())
            .await
        {
            Err(e) => error!("Failed to send sphinx packet to the gateway! - {:?}", e),
            Ok(was_successful) if !was_successful => {
                warn!("Sent sphinx packet to the gateway but it failed to get processed!")
            }
            Ok(was_successful) if was_successful => {
                trace!("Successfully forwarded sphinx packet to the gateway!")
            }
            Ok(_) => unreachable!("to shut up the compiler because all patterns ARE covered"),
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(mix_message) = self.mix_rx.next().await {
            self.on_message(mix_message).await;
        }
    }

    pub(crate) fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            self.run().await;
        })
    }
}
