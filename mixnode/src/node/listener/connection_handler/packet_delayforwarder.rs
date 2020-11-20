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
use log::*;
use nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue};
use nymsphinx::forwarding::packet::MixPacket;
use tokio::stream::StreamExt;
use tokio::time::{Duration, Error as TimeError, Instant};

// Delay + MixPacket vs Instant + MixPacket

// rather than using Duration directly, we use an Instant, this way we minimise skew due to
// time packet spent waiting in the queue to get delayed
type PacketSender = mpsc::UnboundedSender<(MixPacket, Instant)>;
type PacketReceiver = mpsc::UnboundedReceiver<(MixPacket, Instant)>;

/// Entity responsible for delaying received sphinx packet and forwarding it to next node.
pub(crate) struct DelayForwarder {
    delay_queue: NonExhaustiveDelayQueue<MixPacket>,
    mixnet_client: mixnet_client::Client,
    packet_sender: PacketSender,
    packet_receiver: PacketReceiver,
}

impl DelayForwarder {
    pub(crate) fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_reconnection_attempts: u32,
    ) -> Self {
        let client_config = mixnet_client::Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_reconnection_attempts,
        );

        let (packet_sender, packet_receiver) = mpsc::unbounded();

        DelayForwarder {
            delay_queue: NonExhaustiveDelayQueue::new(),
            mixnet_client: mixnet_client::Client::new(client_config),
            packet_sender,
            packet_receiver,
        }
    }

    pub(crate) fn sender(&self) -> PacketSender {
        self.packet_sender.clone()
    }

    /// Upon packet being finished getting delayed, forward it to the mixnet.
    async fn handle_done_delaying(
        &mut self,
        packet: Option<Result<Expired<MixPacket>, TimeError>>,
    ) {
        // those are critical errors that I don't think can be recovered from.
        let delayed = packet.expect("the queue has unexpectedly terminated!");
        let delayed_packet = delayed
            .expect("Encountered timer issue within the runtime!")
            .into_inner();

        let next_hop = delayed_packet.next_hop();
        let packet_mode = delayed_packet.packet_mode();
        let sphinx_packet = delayed_packet.into_sphinx_packet();

        if let Err(err) = self
            .mixnet_client
            .send(next_hop, sphinx_packet, packet_mode, false)
            .await
        {
            debug!("failed to forward the packet to {} - {}", next_hop, err)
        }
    }

    fn handle_new_packet(&mut self, new_packet: (MixPacket, Instant)) {
        self.delay_queue.insert_at(new_packet.0, new_packet.1);
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                delayed = self.delay_queue.next() => {
                    self.handle_done_delaying(delayed).await;
                }
                new_packet = self.packet_receiver.next() => {
                    // this one is impossible to ever panic - the object itself contains a sender
                    // and hence it can't happen that ALL senders are dropped
                    self.handle_new_packet(new_packet.unwrap())
                }
            }
        }
    }
}
