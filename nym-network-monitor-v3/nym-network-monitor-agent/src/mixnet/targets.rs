// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::tested_node::TestedNodeDetails;
use crate::mixnet::events::IngressEventsSender;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{
    NoiseConfig, NoiseNetworkView, NoiseNode, NoiseVersion, VersionedNoiseKeyV1,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

/// One target of a wave, paired with the channel its events are to be delivered on.
///
/// The pairing exists because a target's channel is created by whoever owns that target's probe, so
/// the two only meet when the wave's ingress is assembled.
pub(crate) struct WaveTarget {
    pub(crate) node: TestedNodeDetails,
    pub(crate) events: IngressEventsSender,
}

/// What the shared ingress needs to know about ONE target of a wave.
pub(crate) struct IngressTarget {
    /// The target's static Noise public key.
    ///
    /// Held on this side only to build the per-connection responder config. The responder does NOT
    /// authenticate the initiator with it - `perform_responder_handshake` reads just our own keypair
    /// out of the config - but `upgrade_noise_responder` gates on whether the config's view knows
    /// the source address at all, silently falling back to plain TCP on a miss, which then fails as
    /// "not speaking noise". Building that view per connection from the entry we already resolved is
    /// what keeps the gate and the routing from becoming two sources of truth.
    pub(crate) noise_key: x25519::PublicKey,

    /// Where this target's events go. Each target has its own channel, because each target's probe
    /// sequence awaits its own replies.
    pub(crate) events: IngressEventsSender,
}

impl IngressTarget {
    /// The Noise config for ONE inbound connection from this target.
    ///
    /// Scoped to the single source it was resolved from, so the responder gate can only pass for the
    /// address we already accepted. A wave-wide view would be the same membership set as
    /// [`WaveIngress`] itself, kept in a second place.
    pub(crate) fn responder_config(
        &self,
        source: IpAddr,
        local_key: Arc<x25519::KeyPair>,
        handshake_timeout: Duration,
    ) -> NoiseConfig {
        let node = NoiseNode::new_nym_node(VersionedNoiseKeyV1 {
            supported_version: NoiseVersion::V1,
            x25519_pubkey: self.noise_key,
        });
        let view = NoiseNetworkView::new(HashMap::from([(source.to_canonical(), node)]));

        NoiseConfig::new(local_key, view, handshake_timeout)
    }
}

/// The shared ingress's view of a wave: every address any target is known by, mapped to the target
/// it belongs to.
///
/// One table, two jobs, which is why it is a type rather than two structures. Both fall out of a
/// single [`target`](Self::target) lookup, so there is no separate membership predicate:
///
/// - an unresolvable source is the wave's known-source set saying no, and is refused at accept time
///   before it can consume a handshake
/// - a resolved entry is how a returned packet is attributed to a target, and what the
///   per-connection responder config is built from
///
/// Keys are CANONICALISED, since a dual-stack listener reports ipv4 peers in their ipv4-mapped form.
///
/// Attribution by source address is sound because node addresses are unique across the node
/// population, unlike agent addresses, which may share an ip and be told apart only by port. Two
/// targets announcing the same address are deliberately NOT handled: the later one wins the slot,
/// the other then presents an unexpected static key on its return connection, fails the handshake
/// and scores zero while its twin scores normally. That is the intended outcome for a misconfigured
/// pair, and it is self-diagnosing from the results.
pub(crate) struct WaveIngress {
    by_source: HashMap<IpAddr, IngressTarget>,
}

impl WaveIngress {
    /// Builds the table from each target of the wave and the channel its events should go to.
    ///
    /// A target contributes an entry under EVERY address it is known by, not just the one it was
    /// assigned: a node may be multi-homed, or be reached over one family and reply over another.
    pub(crate) fn new(targets: &[WaveTarget]) -> Self {
        let mut by_source = HashMap::new();
        for WaveTarget { node, events } in targets {
            for source in &node.known_ips {
                by_source.insert(
                    source.to_canonical(),
                    IngressTarget {
                        noise_key: node.noise_key,
                        events: events.clone(),
                    },
                );
            }
        }

        WaveIngress { by_source }
    }

    /// The target a connection from `source` belongs to, or `None` if no target is known by it.
    pub(crate) fn target(&self, source: IpAddr) -> Option<&IngressTarget> {
        self.by_source.get(&source.to_canonical())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet::events::IngressEvent;
    use crate::mixnet::test_fixtures::{ProbedTarget, ip, socket};

    // a node may be multi-homed, or be reached over one family and reply over another, so the return
    // connection has to be attributed to it from ANY address it is known by
    #[test]
    fn every_address_of_a_target_resolves_to_that_target() {
        let alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1"), ip("aaaa::1")]);
        let bob = ProbedTarget::new(socket("2.2.2.2:1789"), &[ip("2.2.2.2")]);
        let ingress = WaveIngress::new(&[alice.wave_target(), bob.wave_target()]);

        for source in ["1.1.1.1", "aaaa::1"] {
            let resolved = ingress
                .target(ip(source))
                .expect("alice was not resolvable");
            assert_eq!(resolved.noise_key, alice.noise_key(), "{source}");
        }

        let resolved = ingress
            .target(ip("2.2.2.2"))
            .expect("bob was not resolvable");
        assert_eq!(resolved.noise_key, bob.noise_key());
    }

    // a dual-stack listener reports ipv4 peers in their ipv4-mapped form
    #[test]
    fn an_ipv4_mapped_source_resolves_to_its_canonical_target() {
        let alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let ingress = WaveIngress::new(&[alice.wave_target()]);

        let resolved = ingress
            .target(ip("::ffff:1.1.1.1"))
            .expect("the mapped form did not resolve");
        assert_eq!(resolved.noise_key, alice.noise_key());
    }

    #[test]
    fn a_source_no_target_is_known_by_resolves_to_nothing() {
        let alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let ingress = WaveIngress::new(&[alice.wave_target()]);

        // an unresolvable source is what the listener refuses on, so this is the membership check
        assert!(ingress.target(ip("9.9.9.9")).is_none());
    }

    // resolving to the right entry is only half of attribution: the entry has to carry the channel
    // of that target and no other, since a wave's targets each await their own replies
    #[test]
    fn a_routed_event_reaches_only_its_own_targets_channel() {
        let mut alice = ProbedTarget::new(socket("1.1.1.1:1789"), &[ip("1.1.1.1")]);
        let mut bob = ProbedTarget::new(socket("2.2.2.2:1789"), &[ip("2.2.2.2")]);
        let ingress = WaveIngress::new(&[alice.wave_target(), bob.wave_target()]);

        ingress
            .target(ip("1.1.1.1"))
            .expect("alice was not resolvable")
            .events
            .unbounded_send(IngressEvent::HandshakeCompleted(Duration::from_millis(7)))
            .expect("alice's channel was closed");

        match alice.drain().as_slice() {
            [IngressEvent::HandshakeCompleted(took)] => {
                assert_eq!(*took, Duration::from_millis(7))
            }
            _ => panic!("alice did not receive her own handshake event"),
        }
        assert!(
            bob.drain().is_empty(),
            "bob received an event addressed to alice"
        );
    }
}
