// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::topology_control::TopologyAccessor;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::cover::generate_loop_cover_packet;
use nymsphinx::utils::sample_poisson_duration;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::pin::Pin;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time;

pub struct LoopCoverTrafficStream<R>
where
    R: CryptoRng + Rng,
{
    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: time::Duration,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: time::Duration,

    /// Average delay between sending subsequent cover packets.
    average_cover_message_sending_delay: time::Duration,

    /// Internal state, determined by `average_message_sending_delay`,
    /// used to keep track of when a next packet should be sent out.
    next_delay: Pin<Box<time::Sleep>>,

    /// Channel used for sending prepared sphinx packets to `MixTrafficController` that sends them
    /// out to the network without any further delays.
    mix_tx: BatchMixMessageSender,

    /// Represents full address of this client.
    our_full_destination: Recipient,

    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Accessor to the common instance of network topology.
    topology_access: TopologyAccessor,
}

impl<R> Stream for LoopCoverTrafficStream<R>
where
    R: CryptoRng + Rng + Unpin,
{
    // Item is only used to indicate we should create a new message rather than actual cover message
    // reason being to not introduce unnecessary complexity by having to keep state of topology
    // mutex when trying to acquire it. So right now the Stream trait serves as a glorified timer.
    // Perhaps this should be changed in the future.
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if self.next_delay.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        };

        // we know it's time to send a message, so let's prepare delay for the next one
        // Get the `now` by looking at the current `delay` deadline
        let avg_delay = self.average_cover_message_sending_delay;
        let now = self.next_delay.deadline();
        let next_poisson_delay = sample_poisson_duration(&mut self.rng, avg_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let next = now + next_poisson_delay;
        self.next_delay.as_mut().reset(next);

        Poll::Ready(Some(()))
    }
}

// obviously when we finally make shared rng that is on 'higher' level, this should become
// generic `R`
impl LoopCoverTrafficStream<OsRng> {
    pub fn new(
        ack_key: Arc<AckKey>,
        average_ack_delay: time::Duration,
        average_packet_delay: time::Duration,
        average_cover_message_sending_delay: time::Duration,
        mix_tx: BatchMixMessageSender,
        our_full_destination: Recipient,
        topology_access: TopologyAccessor,
    ) -> Self {
        let rng = OsRng;

        LoopCoverTrafficStream {
            ack_key,
            average_ack_delay,
            average_packet_delay,
            average_cover_message_sending_delay,
            next_delay: Box::pin(time::sleep(Default::default())),
            mix_tx,
            our_full_destination,
            rng,
            topology_access,
        }
    }

    async fn on_new_message(&mut self) {
        trace!("next cover message!");

        // TODO for way down the line: in very rare cases (during topology update) we might have
        // to wait a really tiny bit before actually obtaining the permit hence messing with our
        // poisson delay, but is it really a problem?
        let topology_permit = self.topology_access.get_read_permit().await;
        // the ack is sent back to ourselves (and then ignored)
        let topology_ref_option = topology_permit.try_get_valid_topology_ref(
            &self.our_full_destination,
            Some(&self.our_full_destination),
        );
        if topology_ref_option.is_none() {
            warn!("No valid topology detected - won't send any loop cover message this time");
            return;
        }
        let topology_ref = topology_ref_option.unwrap();

        let cover_message = generate_loop_cover_packet(
            &mut self.rng,
            topology_ref,
            &self.ack_key,
            &self.our_full_destination,
            self.average_ack_delay,
            self.average_packet_delay,
        )
        .expect("Somehow failed to generate a loop cover message with a valid topology");

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        self.mix_tx.unbounded_send(vec![cover_message]).unwrap();

        // TODO: I'm not entirely sure whether this is really required, because I'm not 100%
        // sure how `yield_now()` works - whether it just notifies the scheduler or whether it
        // properly blocks. So to play it on the safe side, just explicitly drop the read permit
        drop(topology_permit);

        // JS: due to identical logical structure to OutQueueControl::on_message(), this is also
        // presumably required to prevent bugs in the future. Exact reason is still unknown to me.
        tokio::task::yield_now().await;
    }

    async fn run(&mut self) {
        // we should set initial delay only when we actually start the stream
        self.next_delay = Box::pin(time::sleep(sample_poisson_duration(
            &mut self.rng,
            self.average_cover_message_sending_delay,
        )));

        while self.next().await.is_some() {
            self.on_new_message().await;
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }
}
