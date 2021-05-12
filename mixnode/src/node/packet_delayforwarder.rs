// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::metrics::MetricsReporter;
use futures::channel::mpsc;
use futures::StreamExt;
use log::debug;
use nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue, TimerError};
use nymsphinx::forwarding::packet::MixPacket;
use tokio::time::{Duration, Instant};

// Delay + MixPacket vs Instant + MixPacket

// rather than using Duration directly, we use an Instant, this way we minimise skew due to
// time packet spent waiting in the queue to get delayed
pub(crate) type PacketDelayForwardSender = mpsc::UnboundedSender<(MixPacket, Option<Instant>)>;
type PacketDelayForwardReceiver = mpsc::UnboundedReceiver<(MixPacket, Option<Instant>)>;

/// Entity responsible for delaying received sphinx packet and forwarding it to next node.
pub(crate) struct DelayForwarder {
    delay_queue: NonExhaustiveDelayQueue<MixPacket>,
    mixnet_client: mixnet_client::Client,
    packet_sender: PacketDelayForwardSender,
    packet_receiver: PacketDelayForwardReceiver,
    metrics_reporter: MetricsReporter,
}

impl DelayForwarder {
    pub(crate) fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_connection_buffer_size: usize,
        metrics_reporter: MetricsReporter,
    ) -> Self {
        let client_config = mixnet_client::Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_connection_buffer_size,
        );

        let (packet_sender, packet_receiver) = mpsc::unbounded();

        DelayForwarder {
            delay_queue: NonExhaustiveDelayQueue::new(),
            mixnet_client: mixnet_client::Client::new(client_config),
            packet_sender,
            packet_receiver,
            metrics_reporter,
        }
    }

    pub(crate) fn sender(&self) -> PacketDelayForwardSender {
        self.packet_sender.clone()
    }

    fn forward_packet(&mut self, packet: MixPacket) {
        let next_hop = packet.next_hop();
        let packet_mode = packet.packet_mode();
        let sphinx_packet = packet.into_sphinx_packet();

        if let Err(err) =
            self.mixnet_client
                .send_without_response(next_hop, sphinx_packet, packet_mode)
        {
            debug!("failed to forward the packet to {} - {}", next_hop, err)
        } else {
            self.metrics_reporter.report_sent(next_hop.to_string());
        }
    }

    /// Upon packet being finished getting delayed, forward it to the mixnet.
    fn handle_done_delaying(&mut self, packet: Option<Result<Expired<MixPacket>, TimerError>>) {
        // those are critical errors that I don't think can be recovered from.
        let delayed = packet.expect("the queue has unexpectedly terminated!");
        let delayed_packet = delayed
            .expect("Encountered timer issue within the runtime!")
            .into_inner();

        self.forward_packet(delayed_packet)
    }

    fn handle_new_packet(&mut self, new_packet: (MixPacket, Option<Instant>)) {
        // in case of a zero delay packet, don't bother putting it in the delay queue,
        // just forward it immediately
        if let Some(instant) = new_packet.1 {
            // check if the delay has already expired, if so, don't bother putting it through
            // the delay queue only to retrieve it immediately. Just forward it.
            if instant.checked_duration_since(Instant::now()).is_none() {
                self.forward_packet(new_packet.0)
            } else {
                self.delay_queue.insert_at(new_packet.0, instant);
            }
        } else {
            self.forward_packet(new_packet.0)
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                delayed = self.delay_queue.next() => {
                    self.handle_done_delaying(delayed);
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
