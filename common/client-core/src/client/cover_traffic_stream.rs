// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::topology_control::TopologyAccessor;
// use crate::client::COVER_PACKETS_SENT;
use crate::{config, spawn_future};
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::cover::generate_loop_cover_packet;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_sphinx::utils::sample_poisson_duration;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{sleep, Sleep};

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::{sleep, Sleep};

use super::packet_statistics_control::{PacketStatisticsEvent, PacketStatisticsReporter};

pub struct LoopCoverTrafficStream<R>
where
    R: CryptoRng + Rng,
{
    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Defines configuration options related to cover traffic.
    cover_traffic: config::CoverTraffic,

    /// Internal state, determined by `average_message_sending_delay`,
    /// used to keep track of when a next packet should be sent out.
    next_delay: Pin<Box<Sleep>>,

    /// Channel used for sending prepared nym packets to `MixTrafficController` that sends them
    /// out to the network without any further delays.
    mix_tx: BatchMixMessageSender,

    /// Represents full address of this client.
    our_full_destination: Recipient,

    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Accessor to the common instance of network topology.
    topology_access: TopologyAccessor,

    /// Primary predefined packet size used for the loop cover messages.
    primary_packet_size: PacketSize,

    /// Optional secondary predefined packet size used for the loop cover messages.
    secondary_packet_size: Option<PacketSize>,

    packet_type: PacketType,

    stats_tx: PacketStatisticsReporter,
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
        let avg_delay = self.cover_traffic.loop_cover_traffic_average_delay;
        let next_poisson_delay = sample_poisson_duration(&mut self.rng, avg_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let now = self.next_delay.deadline();
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
        average_ack_delay: Duration,
        mix_tx: BatchMixMessageSender,
        our_full_destination: Recipient,
        topology_access: TopologyAccessor,
        traffic_config: config::Traffic,
        cover_config: config::CoverTraffic,
        stats_tx: PacketStatisticsReporter,
    ) -> Self {
        let rng = OsRng;

        let next_delay = Box::pin(sleep(Default::default()));

        LoopCoverTrafficStream {
            ack_key,
            average_ack_delay,
            cover_traffic: cover_config,
            next_delay,
            mix_tx,
            our_full_destination,
            rng,
            topology_access,
            primary_packet_size: traffic_config.primary_packet_size,
            secondary_packet_size: traffic_config.secondary_packet_size,
            packet_type: traffic_config.packet_type,
            stats_tx,
        }
    }

    fn set_next_delay(&mut self, amount: Duration) {
        let next_delay = Box::pin(sleep(amount));
        self.next_delay = next_delay;
    }

    fn loop_cover_message_size(&mut self) -> PacketSize {
        let Some(secondary_packet_size) = self.secondary_packet_size else {
            return self.primary_packet_size;
        };

        let use_primary = self
            .rng
            .gen_bool(self.cover_traffic.cover_traffic_primary_size_ratio);

        if use_primary {
            self.primary_packet_size
        } else {
            secondary_packet_size
        }
    }

    async fn on_new_message(&mut self) {
        trace!("next cover message!");

        let cover_traffic_packet_size = self.loop_cover_message_size();
        trace!("the next loop cover message will be put in a {cover_traffic_packet_size} packet");

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
            self.cover_traffic.loop_cover_traffic_average_delay,
            cover_traffic_packet_size,
            self.packet_type,
        )
        .expect("Somehow failed to generate a loop cover message with a valid topology");

        if let Err(err) = self.mix_tx.try_send(vec![cover_message]) {
            match err {
                TrySendError::Full(_) => {
                    // This isn't a problem, if the channel is full means we're already sending the
                    // max amount of messages downstream can handle.
                    log::debug!("Failed to send cover message - channel full");
                }
                TrySendError::Closed(_) => {
                    log::warn!("Failed to send cover message - channel closed");
                }
            }
        } else {
            if self
                .stats_tx
                .send(PacketStatisticsEvent::CoverPacketSent)
                .is_err()
            {
                log::error!("Failed to send cover packet statistics event - channel closed");
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

    pub fn start_with_shutdown(mut self, mut shutdown: nym_task::TaskClient) {
        if self.cover_traffic.disable_loop_cover_traffic_stream {
            // we should have never got here in the first place - the task should have never been created to begin with
            // so panic and review the code that lead to this branch
            panic!("attempted to start LoopCoverTrafficStream while config explicitly disabled it.")
        }

        // we should set initial delay only when we actually start the stream
        let sampled = sample_poisson_duration(
            &mut self.rng,
            self.cover_traffic.loop_cover_traffic_average_delay,
        );
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
