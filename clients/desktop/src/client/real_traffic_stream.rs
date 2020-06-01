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

use crate::client::inbound_messages::InputMessage;
use crate::client::mix_traffic::MixMessage;
use crate::client::topology_control::TopologyAccessor;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::{error, info, trace, warn};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::utils::{encapsulation, poisson};
use std::pin::Pin;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::time;
use topology::NymTopology;

pub(crate) struct OutQueueControl<T: NymTopology> {
    average_packet_delay: Duration,
    average_message_sending_delay: Duration,
    next_delay: time::Delay,
    mix_tx: mpsc::UnboundedSender<MixMessage>,
    input_rx: mpsc::UnboundedReceiver<InputMessage>,
    our_full_destination: Recipient,
    topology_access: TopologyAccessor<T>,
}

pub(crate) enum StreamMessage {
    Cover,
    Real(InputMessage),
}

impl<T: NymTopology> Stream for OutQueueControl<T> {
    type Item = StreamMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if Pin::new(&mut self.next_delay).poll(cx).is_pending() {
            return Poll::Pending;
        };

        // we know it's time to send a message, so let's prepare delay for the next one
        // Get the `now` by looking at the current `delay` deadline
        let now = self.next_delay.deadline();
        let next_poisson_delay = poisson::sample(self.average_message_sending_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let next = now + next_poisson_delay;
        self.next_delay.reset(next);

        // decide what kind of message to send
        match Pin::new(&mut self.input_rx).poll_next(cx) {
            // in the case our real message channel stream was closed, we should also indicate we are closed
            // (and whoever is using the stream should panic)
            Poll::Ready(None) => Poll::Ready(None),

            // if there's an actual message - return it
            Poll::Ready(Some(real_message)) => Poll::Ready(Some(StreamMessage::Real(real_message))),

            // otherwise construct a dummy one
            Poll::Pending => Poll::Ready(Some(StreamMessage::Cover)),
        }
    }
}

impl<T: 'static + NymTopology> OutQueueControl<T> {
    pub(crate) fn new(
        mix_tx: mpsc::UnboundedSender<MixMessage>,
        input_rx: mpsc::UnboundedReceiver<InputMessage>,
        our_full_destination: Recipient,
        topology_access: TopologyAccessor<T>,
        average_packet_delay: Duration,
        average_message_sending_delay: Duration,
    ) -> Self {
        OutQueueControl {
            average_packet_delay,
            average_message_sending_delay,
            next_delay: time::delay_for(Default::default()),
            mix_tx,
            input_rx,
            our_full_destination,
            topology_access,
        }
    }

    async fn get_route(&self, other_recipient: Option<&Recipient>) -> Option<Vec<nymsphinx::Node>> {
        match other_recipient {
            // we are sending to ourselves
            None => {
                self.topology_access
                    .random_route_to_gateway(&self.our_full_destination.gateway())
                    .await
            }
            Some(other_recipient) => {
                self.topology_access
                    .random_route_to_gateway(&other_recipient.gateway())
                    .await
            }
        }
    }

    async fn on_message(&mut self, next_message: StreamMessage) {
        trace!("created new message");

        let next_packet = match next_message {
            StreamMessage::Cover => {
                let route = self.get_route(None).await;
                if route.is_none() {
                    warn!("No valid topology detected - won't send any real or loop message this time");
                    return;
                }
                let route = route.unwrap();
                encapsulation::loop_cover_message_route(
                    self.our_full_destination.destination().clone(),
                    route,
                    self.average_packet_delay,
                )
            }
            StreamMessage::Real(real_message) => {
                let (recipient, data) = real_message.destruct();
                let route = self.get_route(Some(&recipient)).await;
                if route.is_none() {
                    warn!("No valid topology detected - won't send any real or loop message this time");
                    return;
                }
                let route = route.unwrap();
                encapsulation::encapsulate_message_route(
                    recipient.destination(),
                    data,
                    route,
                    self.average_packet_delay,
                )
            }
        };

        let next_packet = match next_packet {
            Ok(message) => message,
            Err(err) => {
                error!(
                    "Somehow we managed to create an invalid traffic message - {:?}",
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
            .unbounded_send(MixMessage::new(next_packet.0, next_packet.1))
            .unwrap();
        // JS: Not entirely sure why or how it fixes stuff, but without the yield call,
        // the UnboundedReceiver [of mix_rx] will not get a chance to read anything
        // JS2: Basically it was the case that with high enough rate, the stream had already a next value
        // ready and hence was immediately re-scheduled causing other tasks to be starved;
        // yield makes it go back the scheduling queue regardless of its value availability
        tokio::task::yield_now().await;
    }

    pub(crate) async fn run_out_queue_control(mut self) {
        // we should set initial delay only when we actually start the stream
        self.next_delay = time::delay_for(poisson::sample(self.average_message_sending_delay));

        info!("Starting out queue controller...");
        while let Some(next_message) = self.next().await {
            self.on_message(next_message).await;
        }
    }

    pub(crate) fn start(self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move { self.run_out_queue_control().await })
    }
}
