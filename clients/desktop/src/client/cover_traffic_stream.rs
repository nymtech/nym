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

use crate::client::mix_traffic::{MixMessage, MixMessageSender};
use crate::client::topology_control::TopologyAccessor;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nymsphinx::{
    utils::{encapsulation, poisson},
    Destination,
};
use std::pin::Pin;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::time;
use topology::NymTopology;

pub(crate) struct LoopCoverTrafficStream<T: NymTopology> {
    average_packet_delay: Duration,
    average_cover_message_sending_delay: Duration,
    next_delay: time::Delay,
    mix_tx: MixMessageSender,
    our_info: Destination,
    topology_access: TopologyAccessor<T>,
}

impl<T: NymTopology> Stream for LoopCoverTrafficStream<T> {
    // Item is only used to indicate we should create a new message rather than actual cover message
    // reason being to not introduce unnecessary complexity by having to keep state of topology
    // mutex when trying to acquire it. So right now the Stream trait serves as a glorified timer.
    // Perhaps this should be changed in the future.
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if Pin::new(&mut self.next_delay).poll(cx).is_pending() {
            return Poll::Pending;
        };

        // we know it's time to send a message, so let's prepare delay for the next one
        // Get the `now` by looking at the current `delay` deadline
        let now = self.next_delay.deadline();
        let next_poisson_delay = poisson::sample(self.average_cover_message_sending_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let next = now + next_poisson_delay;
        self.next_delay.reset(next);

        Poll::Ready(Some(()))
    }
}

impl<T: 'static + NymTopology> LoopCoverTrafficStream<T> {
    pub(crate) fn new(
        mix_tx: MixMessageSender,
        our_info: Destination,
        topology_access: TopologyAccessor<T>,
        average_cover_message_sending_delay: time::Duration,
        average_packet_delay: time::Duration,
    ) -> Self {
        LoopCoverTrafficStream {
            average_packet_delay,
            average_cover_message_sending_delay,
            next_delay: time::delay_for(Default::default()),
            mix_tx,
            our_info,
            topology_access,
        }
    }

    async fn on_new_message(&mut self) {
        trace!("next cover message!");
        let route = match self.topology_access.random_route().await {
            None => {
                warn!("No valid topology detected - won't send any loop cover message this time");
                return;
            }
            Some(route) => route,
        };

        let cover_message = match encapsulation::loop_cover_message_route(
            self.our_info.address.clone(),
            self.our_info.identifier,
            route,
            self.average_packet_delay,
        ) {
            Ok(message) => message,
            Err(err) => {
                error!(
                    "Somehow we managed to create an invalid cover message - {:?}",
                    err
                );
                return;
            }
        };

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        self.mix_tx
            .unbounded_send(MixMessage::new(cover_message.0, cover_message.1))
            .unwrap();
        // JS: due to identical logical structure to OutQueueControl::on_message(), this is also
        // presumably required to prevent bugs in the future. Exact reason is still unknown to me.
        tokio::task::yield_now().await;
    }

    async fn run(&mut self) {
        // we should set initial delay only when we actually start the stream
        self.next_delay =
            time::delay_for(poisson::sample(self.average_cover_message_sending_delay));

        while let Some(_) = self.next().await {
            self.on_new_message().await;
        }
    }

    pub(crate) fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            self.run().await;
        })
    }
}
