// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::events::{
    IngressEvent, IngressEventsReceiver, IngressEventsSender, ReceivedPacket,
};
use crate::mixnet::sphinx::payload::{PayloadRecovery, ProcessedPacket};
use anyhow::Context;
use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn};

/// The receiving side of ONE target of a wave.
///
/// It owns that target's channel, the strategy for decrypting what arrives on it, and the facts its
/// connection reported. The wave's ingress routes to it by cloning [`events_sender`](Self::events_sender);
/// the probe reads from it one packet at a time with [`next_packet`](Self::next_packet) or in bulk
/// with [`all_available`](Self::all_available).
///
/// Per target rather than per connection, because duplicates and counts are properties of a target's
/// whole result set: a node may reply over more than one connection, and no single connection can
/// tell whether an id has already been seen.
pub(crate) struct TargetInbox {
    /// Decryption strategy: either reuse a pre-built header or perform full sphinx processing.
    payload_recovery: PayloadRecovery,

    /// How long [`next_packet`](Self::next_packet) will wait before returning a timeout error.
    receive_timeout: Duration,

    /// Sender half kept alive so the channel stays open as long as this inbox exists.
    sender: IngressEventsSender,

    /// Receiver half polled by [`next_packet`](Self::next_packet) and [`all_available`](Self::all_available).
    receiver: IngressEventsReceiver,

    /// Duration of this target's ingress Noise handshake, once its connection reported one.
    ingress_handshake: Option<Duration>,

    /// Why this target's connection never became usable, if it connected back and failed.
    ingress_failure: Option<String>,
}

impl TargetInbox {
    /// Creates a new processor along with an internal channel for receiving packets.
    pub(crate) fn new(payload_recovery: PayloadRecovery, receive_timeout: Duration) -> Self {
        let (sender, receiver) = unbounded();

        Self {
            payload_recovery,
            receive_timeout,
            sender,
            receiver,
            ingress_handshake: None,
            ingress_failure: None,
        }
    }

    /// Returns a clone of the sender half, which is what the wave's ingress routes this target's
    /// events to.
    pub(crate) fn events_sender(&self) -> IngressEventsSender {
        self.sender.clone()
    }

    /// Decrypts a [`ReceivedPacket`] and computes its RTT from the embedded send timestamp.
    fn process_received(&self, packet: ReceivedPacket) -> anyhow::Result<ProcessedPacket> {
        let sphinx_packet = packet
            .received
            .into_inner()
            .to_sphinx_packet()
            .context("the received packet was not a sphinx packet!")?;
        let received_content = self.payload_recovery.recover_test_payload(sphinx_packet)?;
        let latency = packet.received_at - received_content.sending_timestamp;

        Ok(ProcessedPacket {
            id: received_content.id,
            rtt: latency.unsigned_abs(),
        })
    }

    /// Drains all packets currently available in the channel without blocking.
    /// Returns a vec of results — decryption failures are included as `Err` entries rather
    /// than causing the entire drain to abort.
    pub(crate) fn all_available(&mut self) -> Vec<anyhow::Result<ProcessedPacket>> {
        let mut packets = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            if let Some(packet) = self.record(event) {
                packets.push(self.process_received(packet));
            }
        }

        debug!("drained {} immediately available packets", packets.len());
        packets
    }

    /// Waits for the next packet, up to `receive_timeout`.
    /// Returns `Err` on timeout, channel exhaustion, or decryption failure.
    ///
    /// The target's channel also carries the facts about its connection, so this loops until a packet
    /// actually arrives rather than treating the first event as one. The timeout bounds the whole
    /// wait, not each event, so a stream of non-packet events cannot extend it.
    pub(crate) async fn next_packet(&mut self) -> anyhow::Result<ProcessedPacket> {
        timeout(self.receive_timeout, async {
            loop {
                let event = self
                    .receiver
                    .next()
                    .await
                    .context("stream has been exhausted")?;

                if let Some(packet) = self.record(event) {
                    return self.process_received(packet);
                }
            }
        })
        .await
        .inspect_err(|_| {
            warn!(
                "timed out waiting for next packet after {}",
                humantime::format_duration(self.receive_timeout)
            )
        })?
    }

    /// Files one event, returning the packet it carried if it was one.
    fn record(&mut self, event: IngressEvent) -> Option<ReceivedPacket> {
        match event {
            IngressEvent::Packet(packet) => Some(packet),
            IngressEvent::HandshakeCompleted(took) => {
                self.ingress_handshake = Some(took);
                None
            }
            IngressEvent::HandshakeFailed(err) => {
                self.ingress_failure = Some(err);
                None
            }
        }
    }

    /// How long this target's return connection took to complete its Noise handshake, if it got that
    /// far.
    pub(crate) fn ingress_handshake(&self) -> Option<Duration> {
        self.ingress_handshake
    }

    /// Why this target's return connection never became usable, if it connected back at all.
    ///
    /// Distinguishes a node that never answered from one that answered and failed the handshake,
    /// which is the difference between a dead node and a stale noise key.
    pub(crate) fn ingress_failure(&self) -> Option<&str> {
        self.ingress_failure.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet::test_fixtures::{ProbedTarget, ip, socket};

    fn processor(target: &ProbedTarget) -> TargetInbox {
        TargetInbox::new(target.payload_recovery(), Duration::from_millis(50))
    }

    fn target() -> ProbedTarget {
        ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")])
    }

    // a target's channel carries the facts about its connection as well as its packets, so the
    // handshake must be filed rather than mistaken for a packet or left to block the probe
    #[tokio::test]
    async fn a_handshake_is_recorded_without_being_taken_for_a_packet() {
        let target = target();
        let mut processor = processor(&target);
        let sender = processor.events_sender();

        sender
            .unbounded_send(IngressEvent::HandshakeCompleted(Duration::from_millis(7)))
            .expect("the processor dropped its channel");
        sender
            .unbounded_send(IngressEvent::Packet(ReceivedPacket::new(target.reply(1))))
            .expect("the processor dropped its channel");

        let packet = processor
            .next_packet()
            .await
            .expect("the packet behind the handshake event was not returned");

        assert_eq!(packet.id, 1);
        assert_eq!(
            processor.ingress_handshake(),
            Some(Duration::from_millis(7))
        );
        assert!(processor.ingress_failure().is_none());
    }

    #[tokio::test]
    async fn a_handshake_failure_is_recorded_against_the_target() {
        let target = target();
        let mut processor = processor(&target);

        processor
            .events_sender()
            .unbounded_send(IngressEvent::HandshakeFailed("stale noise key".to_string()))
            .expect("the processor dropped its channel");

        // nothing will follow a failed handshake, so the probe times out rather than being handed a
        // packet, and the reason is available to explain the empty result
        assert!(processor.next_packet().await.is_err());
        assert_eq!(processor.ingress_failure(), Some("stale noise key"));
        assert!(processor.ingress_handshake().is_none());
    }

    #[test]
    fn a_bulk_drain_files_events_and_returns_only_packets() {
        let target = target();
        let mut processor = processor(&target);
        let sender = processor.events_sender();

        for event in [
            IngressEvent::HandshakeCompleted(Duration::from_millis(3)),
            IngressEvent::Packet(ReceivedPacket::new(target.reply(1))),
            IngressEvent::Packet(ReceivedPacket::new(target.reply(2))),
        ] {
            sender
                .unbounded_send(event)
                .expect("the processor dropped its channel");
        }

        let drained = processor.all_available();

        let ids = drained
            .into_iter()
            .map(|packet| packet.expect("a drained packet failed to decrypt").id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(
            processor.ingress_handshake(),
            Some(Duration::from_millis(3))
        );
    }
}
