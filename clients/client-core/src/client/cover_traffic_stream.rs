// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::topology_control::TopologyAccessor;
use crate::spawn_future;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::cover::generate_loop_cover_packet;
use nymsphinx::params::PacketSize;
use nymsphinx::utils::sample_poisson_duration;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use wasm_timer;

pub struct LoopCoverTrafficStream<R>
where
    R: CryptoRng + Rng,
{
    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay between sending subsequent cover packets.
    average_cover_message_sending_delay: Duration,

    /// Internal state, determined by `average_message_sending_delay`,
    /// used to keep track of when a next packet should be sent out.
    #[cfg(not(target_arch = "wasm32"))]
    next_delay: Pin<Box<time::Sleep>>,

    #[cfg(target_arch = "wasm32")]
    next_delay: Pin<Box<wasm_timer::Delay>>,

    /// Channel used for sending prepared sphinx packets to `MixTrafficController` that sends them
    /// out to the network without any further delays.
    mix_tx: BatchMixMessageSender,

    /// Represents full address of this client.
    our_full_destination: Recipient,

    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Accessor to the common instance of network topology.
    topology_access: TopologyAccessor,

    /// Predefined packet size used for the loop cover messages.
    packet_size: PacketSize,
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
        let next_poisson_delay = sample_poisson_duration(&mut self.rng, avg_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = self.next_delay.deadline();
            let next = now + next_poisson_delay;
            self.next_delay.as_mut().reset(next);
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.next_delay.as_mut().reset(next_poisson_delay);
        }

        Poll::Ready(Some(()))
    }
}

// obviously when we finally make shared rng that is on 'higher' level, this should become
// generic `R`
impl LoopCoverTrafficStream<OsRng> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ack_key: Arc<AckKey>,
        average_ack_delay: Duration,
        average_packet_delay: Duration,
        average_cover_message_sending_delay: Duration,
        mix_tx: BatchMixMessageSender,
        our_full_destination: Recipient,
        topology_access: TopologyAccessor,
    ) -> Self {
        let rng = OsRng;

        #[cfg(not(target_arch = "wasm32"))]
        let next_delay = Box::pin(time::sleep(Default::default()));

        #[cfg(target_arch = "wasm32")]
        let next_delay = Box::pin(wasm_timer::Delay::new(Default::default()));

        LoopCoverTrafficStream {
            ack_key,
            average_ack_delay,
            average_packet_delay,
            average_cover_message_sending_delay,
            next_delay,
            mix_tx,
            our_full_destination,
            rng,
            topology_access,
            packet_size: Default::default(),
        }
    }

    pub fn set_custom_packet_size(&mut self, packet_size: PacketSize) {
        self.packet_size = packet_size;
    }

    fn set_next_delay(&mut self, amount: Duration) {
        #[cfg(not(target_arch = "wasm32"))]
        let next_delay = Box::pin(time::sleep(amount));

        #[cfg(target_arch = "wasm32")]
        let next_delay = Box::pin(wasm_timer::Delay::new(amount));

        self.next_delay = next_delay;
    }

    async fn on_new_message(&mut self) {
        trace!("next cover message!");

        // TODO for way down the line: in very rare cases (during topology update) we might have
        // to wait a really tiny bit before actually obtaining the permit hence messing with our
        // poisson delay, but is it really a problem?
        let topology_permit = self.topology_access.get_read_permit().await;
        // the ack is sent back to ourselves (and then ignored)
        let topology_ref = match topology_permit.try_get_valid_topology_ref(
            &self.our_full_destination,
            Some(&self.our_full_destination),
        ) {
            Ok(topology) => topology,
            Err(err) => {
                warn!("We're not going to send any loop cover message this time, as the current topology seem to be invalid - {err}");
                return;
            }
        };

        let cover_message = generate_loop_cover_packet(
            &mut self.rng,
            topology_ref,
            &self.ack_key,
            &self.our_full_destination,
            self.average_ack_delay,
            self.average_packet_delay,
            self.packet_size,
        )
        .expect("Somehow failed to generate a loop cover message with a valid topology");

        if let Err(err) = self.mix_tx.try_send(vec![cover_message]) {
            match err {
                TrySendError::Full(_) => {
                    // This isn't a problem, if the channel is full means we're already sending the
                    // max amount of messages downstream can handle.
                    log::debug!("Failed to send cover message - channel full");
                    // However it's still useful to alert the user that the gateway or the link to
                    // the gateway can't keep up. Either due to insufficient bandwidth on the
                    // client side, or that the gateway is overloaded.
                    log::warn!("Failed to send: gateway appears to not keep up");
                }
                TrySendError::Closed(_) => {
                    log::warn!("Failed to send cover message - channel closed");
                }
            }
        }

        // TODO: I'm not entirely sure whether this is really required, because I'm not 100%
        // sure how `yield_now()` works - whether it just notifies the scheduler or whether it
        // properly blocks. So to play it on the safe side, just explicitly drop the read permit
        drop(topology_permit);

        // JS: due to identical logical structure to OutQueueControl::on_message(), this is also
        // presumably required to prevent bugs in the future. Exact reason is still unknown to me.

        // TODO: temporary and BAD workaround for wasm (we should find a way to yield here in wasm)
        #[cfg(not(target_arch = "wasm32"))]
        tokio::task::yield_now().await;
    }

    pub fn start_with_shutdown(mut self, mut shutdown: task::ShutdownListener) {
        // we should set initial delay only when we actually start the stream
        let sampled =
            sample_poisson_duration(&mut self.rng, self.average_cover_message_sending_delay);
        self.set_next_delay(sampled);

        spawn_future(async move {
            debug!("Started LoopCoverTrafficStream with graceful shutdown support");

            while !shutdown.is_shutdown() {
                tokio::select! {
                    biased;
                    _ = shutdown.recv() => {
                        log::trace!("LoopCoverTrafficStream: Received shutdown");
                    }
                    next = self.next() => {
                        if next.is_some() {
                            self.on_new_message().await;
                        } else {
                            log::trace!("LoopCoverTrafficStream: Stopping since channel closed");
                            break;
                        }
                    }
                }
            }
            shutdown.recv_timeout().await;
            log::debug!("LoopCoverTrafficStream: Exiting");
        })
    }
}
