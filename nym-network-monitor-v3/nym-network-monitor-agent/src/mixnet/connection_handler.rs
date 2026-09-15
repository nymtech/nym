// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::events::{IngressEvent, IngressEventsSender, ReceivedPacket};
use crate::mixnet::targets::WaveIngress;
use anyhow::bail;
use futures::{Stream, StreamExt};
use nym_crypto::asymmetric::x25519;
use nym_noise::config::NoiseConfig;
use nym_noise::connection::Connection;
use nym_noise::upgrade_noise_responder;
use nym_sphinx_framing::codec::{NymCodec, NymCodecError};
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

/// A mixnet connection once its Noise handshake has completed.
pub(crate) type NoiseMixnetConnection = Framed<Connection<TcpStream>, NymCodec>;

/// One item read off such a connection.
pub(crate) type WaveConnectionItem = Result<FramedNymPacket, NymCodecError>;

/// What every connection handler of a wave needs, cloned into each one.
#[derive(Clone)]
pub(crate) struct SharedHandlerData {
    /// The wave's targets. Resolving a connection against them serves two purposes: refusing a
    /// source no target is known by, and supplying the key for that connection's responder config.
    pub(crate) targets: Arc<WaveIngress>,

    /// The agent's own Noise key pair, which is what the responder handshake authenticates with.
    pub(crate) noise_key: Arc<x25519::KeyPair>,

    /// Timeout applied to each connection's Noise handshake.
    pub(crate) noise_handshake_timeout: Duration,

    pub(super) shutdown_token: ShutdownToken,
}

impl SharedHandlerData {
    pub(crate) fn new(
        targets: Arc<WaveIngress>,
        noise_key: Arc<x25519::KeyPair>,
        noise_handshake_timeout: Duration,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedHandlerData {
            targets,
            noise_key,
            noise_handshake_timeout,
            shutdown_token,
        }
    }
}

/// Reads ONE inbound connection and hands what arrives to the target it came from.
///
/// A handler per connection, rather than one task multiplexing all of them: `upgrade_noise_responder`
/// is async, so upgrading inline anywhere shared would let one slow peer delay every other target of
/// the wave. Attribution needs nothing to be carried per packet, since the source resolves to its
/// target once, before the handshake.
///
/// Fairness across a wave's connections is consequently the runtime's business rather than something
/// this module arranges.
pub(crate) struct ConnectionHandler {
    shared_data: SharedHandlerData,
    source: SocketAddr,
}

impl ConnectionHandler {
    pub(crate) fn new(shared_data: SharedHandlerData, source: SocketAddr) -> Self {
        Self {
            shared_data,
            source,
        }
    }

    /// Refuses, upgrades, then reads, for the lifetime of one connection.
    pub(crate) async fn handle_connection(&self, socket: TcpStream) {
        let source = self.source;

        // resolved ONCE, before the handshake, so that a stranger hitting the port cannot consume
        // one and so that the handshake's outcome is attributable to a target rather than being an
        // anonymous log line
        let Some(target) = self.shared_data.targets.target(source.ip()) else {
            warn!(
                "received a connection from {source}, which is not a target of this wave. ignoring it"
            );
            return;
        };
        let events = target.events.clone();
        let noise_config = target.responder_config(
            source.ip(),
            self.shared_data.noise_key.clone(),
            self.shared_data.noise_handshake_timeout,
        );

        info!("accepted connection from {source}. beginning the noise handshake (responder)");
        let handshake_start = Instant::now();
        let upgraded = self.upgrade_connection(&noise_config, socket).await;
        let took = handshake_start.elapsed();

        if let Some(stream) = self.report_handshake(&events, upgraded, took) {
            self.handle_stream(stream, &events).await
        }
    }

    /// Tells the target how its connection's handshake went, yielding the stream to read if there is
    /// one.
    ///
    /// The outcome is REPORTED rather than only logged, because this is the only place that can tell
    /// "connected back and the crypto did not match" apart from "never answered", and those are
    /// different diagnoses: a stale noise key against a dead node.
    fn report_handshake<S>(
        &self,
        events: &IngressEventsSender,
        upgraded: anyhow::Result<S>,
        took: Duration,
    ) -> Option<S> {
        match upgraded {
            Ok(stream) => self
                .report(events, IngressEvent::HandshakeCompleted(took))
                .then_some(stream),
            Err(err) => {
                let err = format!("{err:#}");
                warn!("{err}");
                self.report(events, IngressEvent::HandshakeFailed(err));
                None
            }
        }
    }

    /// Reads packets off `stream` until it ends or the wave does.
    ///
    /// Generic over the stream so that production reads a [`NoiseMixnetConnection`] while tests read
    /// a plain iterator of packets, which is what keeps attribution testable without a socket.
    async fn handle_stream<S>(&self, mut stream: S, events: &IngressEventsSender)
    where
        S: Stream<Item = WaveConnectionItem> + Unpin,
    {
        let source = self.source;

        loop {
            tokio::select! {
                biased;
                _ = self.shared_data.shutdown_token.cancelled() => {
                    debug!("connection handler for {source}: received shutdown");
                    return;
                }
                next = stream.next() => {
                    let Some(item) = next else {
                        info!("the mixnet connection from {source} was closed");
                        return;
                    };
                    if !self.handle_received_item(events, item) {
                        return;
                    }
                }
            }
        }
    }

    /// Files one item read off the connection, returning whether reading should continue.
    fn handle_received_item(&self, events: &IngressEventsSender, item: WaveConnectionItem) -> bool {
        let packet = match item {
            Ok(packet) => packet,
            Err(err) => {
                // a framing error leaves the stream desynchronised, so the connection is torn down
                // rather than read any further
                error!("failed to read a packet from {}: {err}", self.source);
                return false;
            }
        };

        // stamped HERE, the instant the packet leaves the wire: that stamp is the basis of every
        // round trip figure, so taking it any later would fold this hop's queueing into it
        self.report(events, IngressEvent::Packet(ReceivedPacket::new(packet)))
    }

    /// Sends one event to this connection's target, returning whether anything is still listening.
    fn report(&self, events: &IngressEventsSender, event: IngressEvent) -> bool {
        if events.unbounded_send(event).is_err() {
            debug!(
                "{} has nothing listening for its results any more",
                self.source
            );
            return false;
        }
        true
    }

    /// Performs the responder handshake for this connection.
    ///
    /// The two failure modes are kept distinct: a handshake that errors is a different diagnosis from
    /// one that "succeeded" into a plain TCP connection, which is what the Noise responder does when
    /// it does not recognise the source.
    async fn upgrade_connection(
        &self,
        noise_config: &NoiseConfig,
        socket: TcpStream,
    ) -> anyhow::Result<NoiseMixnetConnection> {
        match upgrade_noise_responder(socket, noise_config).await {
            Ok(stream) if stream.is_noise() => Ok(Framed::new(stream, NymCodec)),
            Ok(_) => bail!(
                "the connection from {} was not upgraded to noise. does the node support the protocol?",
                self.source
            ),
            Err(err) => bail!(
                "failed to upgrade the connection from {} to noise: {err}",
                self.source
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet::targets::WaveTarget;
    use crate::mixnet::test_fixtures::{ProbedTarget, ip, socket};
    use anyhow::anyhow;
    use futures::stream;
    use rand::rngs::OsRng;

    /// What a connection looks like to the handler in a test: a finite stream of already-framed
    /// packets, standing in for the framed noise connection production reads.
    type TestConnection = stream::Iter<std::vec::IntoIter<WaveConnectionItem>>;

    fn connection(packets: Vec<FramedNymPacket>) -> TestConnection {
        stream::iter(packets.into_iter().map(Ok).collect::<Vec<_>>())
    }

    /// A handler for a connection arriving from `source`, over a wave built from `targets`.
    fn handler(source: &str, targets: &[WaveTarget]) -> ConnectionHandler {
        ConnectionHandler::new(
            SharedHandlerData {
                targets: Arc::new(WaveIngress::new(targets)),
                noise_key: Arc::new(x25519::KeyPair::new(&mut OsRng)),
                noise_handshake_timeout: Duration::from_secs(3),
                shutdown_token: ShutdownToken::new(),
            },
            socket(source),
        )
    }

    /// The sender the wave's ingress would route this source's events to.
    fn events_for(handler: &ConnectionHandler) -> IngressEventsSender {
        handler
            .shared_data
            .targets
            .target(handler.source.ip())
            .expect("the source is not a target of this wave")
            .events
            .clone()
    }

    // each connection is read by its own handler, so what proves attribution is that two of them
    // running at once deliver to their own targets and nowhere else. the packets are decrypted with
    // each target's OWN key on the way out, so a misdelivery fails to decrypt rather than merely
    // landing in a bucket whose count happens to match
    #[tokio::test]
    async fn concurrent_connections_deliver_to_their_own_targets() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let mut bob = ProbedTarget::new(socket("2.2.2.2:1789"), &[ip("2.2.2.2")]);
        let wave = [alice.wave_target(), bob.wave_target()];

        let alice_handler = handler("1.1.1.1:1789", &wave);
        let bob_handler = handler("2.2.2.2:1789", &wave);
        let (alice_events, bob_events) = (events_for(&alice_handler), events_for(&bob_handler));

        tokio::join!(
            alice_handler.handle_stream(
                connection(vec![alice.reply(1), alice.reply(2)]),
                &alice_events
            ),
            bob_handler.handle_stream(connection(vec![bob.reply(7)]), &bob_events),
        );

        assert_eq!(alice.received_ids(), vec![1, 2]);
        assert_eq!(bob.received_ids(), vec![7]);
    }

    // a node reached over one family may reply over another, so a connection from a target's OTHER
    // announced address still has to resolve to it
    #[tokio::test]
    async fn a_connection_from_a_targets_other_address_still_reaches_it() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1"), ip("aaaa::1")]);
        let wave = [alice.wave_target()];

        let handler = handler("[aaaa::1]:1789", &wave);
        let events = events_for(&handler);
        handler
            .handle_stream(connection(vec![alice.reply(3)]), &events)
            .await;

        assert_eq!(alice.received_ids(), vec![3]);
    }

    // a framing error desynchronises the stream, so the connection is dropped rather than read on
    #[tokio::test]
    async fn a_framing_error_ends_the_connection() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let wave = [alice.wave_target()];

        let handler = handler("1.1.1.1:1789", &wave);
        let events = events_for(&handler);
        let stream = stream::iter(vec![
            Ok(alice.reply(1)),
            Err(NymCodecError::ToBytes),
            Ok(alice.reply(2)),
        ]);
        handler.handle_stream(stream, &events).await;

        // the packet before the error arrives; the one behind it is never read
        assert_eq!(alice.received_ids(), vec![1]);
    }

    #[tokio::test]
    async fn a_completed_handshake_is_reported_ahead_of_the_packets() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let wave = [alice.wave_target()];

        let handler = handler("1.1.1.1:1789", &wave);
        let events = events_for(&handler);
        let stream = handler
            .report_handshake(
                &events,
                Ok(connection(vec![alice.reply(1)])),
                Duration::from_millis(7),
            )
            .expect("a completed handshake did not yield its stream");
        handler.handle_stream(stream, &events).await;

        match alice.drain().as_slice() {
            [
                IngressEvent::HandshakeCompleted(took),
                IngressEvent::Packet(_),
            ] => {
                assert_eq!(*took, Duration::from_millis(7))
            }
            other => panic!("unexpected events for alice: {} of them", other.len()),
        }
    }

    // telling a stale noise key apart from a node that never answered is the whole reason the
    // outcome is reported to the target rather than only logged
    #[tokio::test]
    async fn a_failed_handshake_is_reported_to_its_own_target() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let mut bob = ProbedTarget::new(socket("2.2.2.2:1789"), &[ip("2.2.2.2")]);
        let wave = [alice.wave_target(), bob.wave_target()];

        let handler = handler("1.1.1.1:1789", &wave);
        let events = events_for(&handler);
        let outcome: anyhow::Result<TestConnection> = Err(anyhow!("responder handshake timed out"));

        assert!(
            handler
                .report_handshake(&events, outcome, Duration::from_millis(1))
                .is_none(),
            "a failed handshake yielded a stream to read"
        );
        match alice.drain().as_slice() {
            [IngressEvent::HandshakeFailed(err)] => assert!(err.contains("timed out")),
            _ => panic!("alice did not receive her own failure"),
        }
        assert!(bob.drain().is_empty(), "bob was told about alice's failure");
    }
}
