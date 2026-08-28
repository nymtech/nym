// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::events::{IngressEvent, ReceivedPacket};
use crate::mixnet::targets::WaveIngress;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::{Stream, StreamExt};
use nym_sphinx_framing::codec::NymCodecError;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_task::ShutdownToken;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamMap;
use tracing::{debug, error, warn};

/// One inbound connection of a wave, as the demux polls it.
pub(crate) type WaveConnection = Result<FramedNymPacket, NymCodecError>;

/// The outcome of one inbound connection's Noise handshake, handed from the listener to the demux.
///
/// The handshake is performed by a task of its own rather than by the demux, and this is what
/// crosses between them. That split is load-bearing: `upgrade_noise_responder` is async, so a
/// handshake awaited inside the demux loop would stall polling for every other target of the wave,
/// which is exactly the serialisation a shared listener exists to remove.
pub(crate) enum UpgradedConnection<S> {
    /// The handshake completed and `stream` can be polled for that target's packets.
    Ready {
        source: IpAddr,
        handshake: Duration,
        stream: S,
    },

    /// The target connected back but the handshake did not produce a usable stream.
    Failed { source: IpAddr, error: String },
}

/// Sender half of the channel carrying handshake outcomes to the demux.
pub(crate) type UpgradedConnectionsSender<S> = UnboundedSender<UpgradedConnection<S>>;

/// Receiver half of the channel carrying handshake outcomes to the demux.
pub(crate) type UpgradedConnectionsReceiver<S> = UnboundedReceiver<UpgradedConnection<S>>;

/// Reads every live connection of a wave and hands each packet to the target it arrived from.
///
/// One task polls all of them through a [`StreamMap`](tokio_stream::StreamMap) keyed by source
/// address, so attribution is the key rather than something a per-connection closure has to carry,
/// a finished connection removes itself, and no single node can starve the others' reads.
///
/// Generic over the connection type so that production instantiates it with the framed noise
/// connection while tests instantiate a plain iterator of packets. That is what puts the select loop,
/// the inserts, the removal-on-end and the routing inside the tested surface, and leaves only the
/// socket accept and the Noise upgrade outside it.
pub(crate) struct IngressDemux<S> {
    targets: Arc<WaveIngress>,

    /// Handshake outcomes still to be taken from the listener.
    upgrades: UpgradedConnectionsReceiver<S>,

    /// Whether any further connection can still arrive. Once the listener has dropped its sender the
    /// wave is bounded by what is already being polled.
    accepting: bool,

    /// The connections currently being polled, keyed by the source each was accepted from.
    live: StreamMap<IpAddr, S>,
}

impl<S> IngressDemux<S>
where
    S: Stream<Item = WaveConnection> + Unpin,
{
    pub(crate) fn new(targets: Arc<WaveIngress>, upgrades: UpgradedConnectionsReceiver<S>) -> Self {
        IngressDemux {
            targets,
            upgrades,
            accepting: true,
            live: StreamMap::new(),
        }
    }

    /// Reports one handshake outcome to the target it concerns and, when it succeeded, starts polling
    /// that connection. An exhausted channel closes the wave to further connections.
    fn handle_connection_upgrade(&mut self, upgrade: Option<UpgradedConnection<S>>) {
        match upgrade {
            Some(UpgradedConnection::Ready {
                source,
                handshake,
                stream,
            }) => {
                self.emit(source, IngressEvent::HandshakeCompleted(handshake));
                // keyed by source, which is what makes every packet off this stream attributable
                // without the connection having to carry anything
                self.live.insert(source, stream);
            }
            Some(UpgradedConnection::Failed { source, error }) => {
                self.emit(source, IngressEvent::HandshakeFailed(error));
            }
            None => self.accepting = false,
        }
    }

    /// Hands one item read off a live connection to the target it arrived from.
    fn handle_live_connection(&mut self, received: Option<(IpAddr, WaveConnection)>) {
        match received {
            Some((source, Ok(packet))) => self.deliver(source, packet),
            Some((source, Err(err))) => {
                // a framing error leaves the stream desynchronised, so the connection is torn down
                // rather than read further, which is what the single-connection listener did by
                // returning out of its read loop
                error!("failed to read a packet from {source}: {err}");
                self.live.remove(&source);
            }
            // the last connection ended between the guard and the poll
            None => (),
        }
    }

    /// Hands one received packet to the target whose address it arrived from.
    ///
    /// The arrival is stamped HERE, the instant the packet comes off the wire, rather than after it
    /// has crossed into the target's channel: that stamp is the basis of every round trip figure, so
    /// taking it any later would fold this hop's queueing delay into the measurement.
    fn deliver(&self, source: IpAddr, packet: FramedNymPacket) {
        self.emit(source, IngressEvent::Packet(ReceivedPacket::new(packet)));
    }

    /// Sends one event to the target `source` resolves to, dropping it if no target is known by that
    /// address or if that target has already finished.
    fn emit(&self, source: IpAddr, event: IngressEvent) {
        let Some(target) = self.targets.target(source) else {
            warn!(
                "received traffic from {source}, which is not a target of this wave. ignoring it"
            );
            return;
        };

        if target.events.unbounded_send(event).is_err() {
            debug!("{source} has nothing listening for its results any more");
        }
    }

    /// Polls handshake outcomes and live connections until the wave is done or shutdown is
    /// signalled.
    ///
    /// Returns when `upgrades` has closed and every inserted stream has ended, so a finite wave
    /// finishes on its own rather than needing to be cancelled.
    pub(crate) async fn run(mut self, shutdown: ShutdownToken) {
        loop {
            // both branches have to be gated: an exhausted channel and an empty map each yield
            // `None` immediately and for ever, so polling either unconditionally is a busy loop.
            // hoisted out of the select so the guards do not borrow alongside the futures
            let accepting = self.accepting;
            let polling = !self.live.is_empty();

            if !accepting && !polling {
                debug!("every connection of the wave has been reported and drained");
                return;
            }

            tokio::select! {
                biased;
                _ = shutdown.cancelled() => {
                    debug!("mixnet demux: received shutdown");
                    return;
                }
                upgrade = self.upgrades.next(), if accepting => {
                    self.handle_connection_upgrade(upgrade)
                }
                received = self.live.next(), if polling => {
                    self.handle_live_connection(received)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet::events::IngressEvent;
    use crate::mixnet::targets::WaveIngress;
    use crate::mixnet::test_fixtures::{ProbedTarget, ip, socket};
    use futures::channel::mpsc::unbounded;
    use futures::stream;

    /// What a connection looks like to the demux in a test: a finite stream of already-framed
    /// packets, standing in for the framed noise connection production hands it.
    type TestConnection = stream::Iter<std::vec::IntoIter<WaveConnection>>;

    fn ready(
        source: IpAddr,
        handshake: Duration,
        packets: Vec<FramedNymPacket>,
    ) -> UpgradedConnection<TestConnection> {
        UpgradedConnection::Ready {
            source,
            handshake,
            stream: stream::iter(packets.into_iter().map(Ok).collect::<Vec<_>>()),
        }
    }

    /// Runs the demux over a fixed set of connections until it has drained them.
    ///
    /// No cancellation is needed and no timing is involved: dropping the sender closes the upgrades
    /// channel and every connection is finite, so the loop has to terminate on its own. A hang here
    /// is a real defect rather than a flaky test.
    async fn drain_wave(ingress: WaveIngress, upgrades: Vec<UpgradedConnection<TestConnection>>) {
        let (tx, rx) = unbounded();
        for upgrade in upgrades {
            tx.unbounded_send(upgrade).expect("upgrades channel closed");
        }
        drop(tx);

        IngressDemux::new(Arc::new(ingress), rx)
            .run(ShutdownToken::new())
            .await;
    }

    #[tokio::test]
    async fn interleaved_replies_are_attributed_to_their_own_target() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let mut bob = ProbedTarget::new(socket("2.2.2.2:1789"), &[ip("2.2.2.2")]);
        let ingress = WaveIngress::new(&[alice.wave_target(), bob.wave_target()]);

        drain_wave(
            ingress,
            vec![
                ready(
                    ip("1.1.1.1"),
                    Duration::from_millis(1),
                    vec![alice.reply(1), alice.reply(2)],
                ),
                ready(ip("2.2.2.2"), Duration::from_millis(1), vec![bob.reply(7)]),
            ],
        )
        .await;

        assert_eq!(alice.received_ids(), vec![1, 2]);
        assert_eq!(bob.received_ids(), vec![7]);
    }

    // a node reached over one family may reply over another, so a target's own second address must
    // reach it rather than being treated as a stranger
    #[tokio::test]
    async fn a_reply_from_a_targets_other_address_still_reaches_it() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1"), ip("aaaa::1")]);
        let ingress = WaveIngress::new(&[alice.wave_target()]);

        drain_wave(
            ingress,
            vec![ready(
                ip("aaaa::1"),
                Duration::from_millis(1),
                vec![alice.reply(3)],
            )],
        )
        .await;

        assert_eq!(alice.received_ids(), vec![3]);
    }

    #[tokio::test]
    async fn a_reply_from_an_unknown_source_is_dropped() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let stranger = ProbedTarget::new(socket("9.9.9.9:1789"), &[ip("9.9.9.9")]);
        // the stranger is deliberately NOT part of the wave
        let ingress = WaveIngress::new(&[alice.wave_target()]);

        drain_wave(
            ingress,
            vec![ready(
                ip("9.9.9.9"),
                Duration::from_millis(1),
                vec![stranger.reply(1)],
            )],
        )
        .await;

        assert!(alice.received_ids().is_empty());
    }

    // the handshake is a fact about that target's connection, so it has to arrive on that target's
    // own stream, and ahead of the packets that could only follow it
    #[tokio::test]
    async fn a_handshake_is_delivered_to_its_target_before_its_packets() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let ingress = WaveIngress::new(&[alice.wave_target()]);

        drain_wave(
            ingress,
            vec![ready(
                ip("1.1.1.1"),
                Duration::from_millis(7),
                vec![alice.reply(1)],
            )],
        )
        .await;

        let events = alice.drain();
        assert!(
            matches!(events.first(), Some(IngressEvent::HandshakeCompleted(took)) if *took == Duration::from_millis(7)),
            "the handshake did not arrive first"
        );
        assert!(matches!(events.get(1), Some(IngressEvent::Packet(_))));
        assert_eq!(events.len(), 2);
    }

    // distinguishing "connected back but the crypto did not match" from silence is the whole point
    // of resolving the source before the handshake
    #[tokio::test]
    async fn a_failed_handshake_is_delivered_to_its_own_target() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let mut bob = ProbedTarget::new(socket("2.2.2.2:1789"), &[ip("2.2.2.2")]);
        let ingress = WaveIngress::new(&[alice.wave_target(), bob.wave_target()]);

        drain_wave(
            ingress,
            vec![UpgradedConnection::Failed {
                source: ip("1.1.1.1"),
                error: "responder handshake timed out".to_string(),
            }],
        )
        .await;

        let events = alice.drain();
        assert!(
            matches!(events.first(), Some(IngressEvent::HandshakeFailed(err)) if err.contains("timed out")),
            "alice did not receive her own failure"
        );
        assert!(bob.drain().is_empty(), "bob was told about alice's failure");
    }
}
