// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use arc_swap::ArcSwap;
use nym_crypto::asymmetric::x25519;
use snow::params::NoiseParams;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use strum_macros::{EnumIter, FromRepr};
use tokio::sync::{Mutex, MutexGuard};

pub use nym_noise_keys::{NoiseVersion, VersionedNoiseKeyV1};

#[derive(Default, Debug, Clone, Copy, EnumIter, FromRepr, Eq, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum NoisePattern {
    #[default]
    XKpsk3 = 1,
    IKpsk2 = 2,
}

impl NoisePattern {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::XKpsk3 => "Noise_XKpsk3_25519_AESGCM_SHA256",
            Self::IKpsk2 => "Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s", //Wireguard handshake (not exactly though)
        }
    }

    // SAFETY: we have tests to ensure that hardcoded pattern are correct
    #[allow(clippy::unwrap_used)]
    pub(crate) fn psk_position(&self) -> u8 {
        //automatic parsing, works for correct pattern, more convenient
        match self.as_str().find("psk") {
            Some(n) => {
                let psk_index = n + 3;
                let psk_char = self.as_str().chars().nth(psk_index).unwrap();
                psk_char.to_string().parse().unwrap()
            }
            None => 0,
        }
    }

    // SAFETY : we have tests to ensure that hardcoded pattern are correct
    #[allow(clippy::unwrap_used)]
    pub(crate) fn as_noise_params(&self) -> NoiseParams {
        self.as_str().parse().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct NoiseNetworkView {
    inner: Arc<NoiseNetworkViewInner>,
}

/// Inner state of [`NoiseNetworkView`], shared behind an `Arc`.
///
/// # Concurrency model
///
/// Reads (on the packet-processing hot path) use `ArcSwap` and are fully lock-free.
/// Writers must first acquire `update_lock` to serialise concurrent updates, then call
/// `swap_view` to atomically publish the new map.  The lock is intentionally *not* wrapping
/// the map itself so that readers are never blocked.
#[derive(Debug)]
struct NoiseNetworkViewInner {
    update_lock: Mutex<()>,
    nodes: ArcSwap<HashMap<IpAddr, NoiseNode>>,
}

/// A node in the noise network map, keyed by IP address.
///
/// A single IP can correspond to either one nym-node (which has a single noise key)
/// or one-or-more network monitor agents (each with its own port and noise key).
/// The two variants have independent lifecycles: nym-node entries come from the
/// nym-api topology refresher, while agent entries come from blockchain events.
#[derive(Debug, Clone)]
pub enum NoiseNode {
    NymNode { key: VersionedNoiseKeyV1 },
    // due to the structure of network monitor agents,
    // it is possible to have multiple destinations with the same host ip address,
    // but a different noise key.
    // however, we are also guaranteed all of those are going to have a unique port.
    // note: we're not storing it in a map, since at maximum we might have maybe 20 or so
    // entries under a single ip address and linear look-up of a vec is faster than the overhead of a hashmap
    NetworkMonitorAgent { nodes: Vec<NetworkMonitorAgentNode> },
}

impl NoiseNode {
    pub fn new_nym_node(key: VersionedNoiseKeyV1) -> Self {
        NoiseNode::NymNode { key }
    }

    pub fn new_agent(socket_addr: SocketAddr, key: VersionedNoiseKeyV1) -> Self {
        NoiseNode::NetworkMonitorAgent {
            nodes: vec![NetworkMonitorAgentNode {
                port: socket_addr.port(),
                key,
            }],
        }
    }

    pub fn is_nym_node(&self) -> bool {
        matches!(self, NoiseNode::NymNode { .. })
    }
}

/// A single network monitor agent identified by its port on a shared host.
///
/// Multiple agents may share an IP address but are guaranteed to have unique ports.
#[derive(Debug, Clone)]
pub struct NetworkMonitorAgentNode {
    pub port: u16,
    pub key: VersionedNoiseKeyV1,
}

impl NoiseNetworkView {
    pub fn new(nodes: HashMap<IpAddr, NoiseNode>) -> Self {
        // ensure we're always storing canonical IPs
        NoiseNetworkView {
            inner: Arc::new(NoiseNetworkViewInner {
                update_lock: Mutex::new(()),
                nodes: ArcSwap::from_pointee(
                    nodes
                        .into_iter()
                        .map(|(k, v)| (k.to_canonical(), v))
                        .collect(),
                ),
            }),
        }
    }

    pub fn new_empty() -> Self {
        Self::new(Default::default())
    }

    /// Build a noise view pre-populated with network monitor agents (used at startup).
    pub fn new_with_agents(agents: HashMap<IpAddr, Vec<NetworkMonitorAgentNode>>) -> Self {
        let mut nodes = HashMap::new();
        for (ip, agent_nodes) in agents {
            nodes.insert(ip, NoiseNode::NetworkMonitorAgent { nodes: agent_nodes });
        }
        Self::new(nodes)
    }

    pub async fn get_update_permit(&self) -> MutexGuard<'_, ()> {
        self.inner.update_lock.lock().await
    }

    /// Atomically replace the noise key map.
    ///
    /// # Precondition
    ///
    /// The caller **must** hold the permit returned by [`NoiseNetworkView::get_update_permit`].
    /// Passing the `MutexGuard` by value enforces this at the type level — the guard is dropped
    /// (releasing the lock) only after the swap completes, preventing torn writes from concurrent
    /// update calls.
    pub fn swap_view(&self, _permit: MutexGuard<'_, ()>, new: HashMap<IpAddr, NoiseNode>) {
        // defensive: ensure stored keys are always canonical so lookups (which canonicalise)
        // always match. callers should still canonicalise before assembling `new` to keep
        // collision resolution deterministic.
        let canonical = new
            .into_iter()
            .map(|(k, v)| (k.to_canonical(), v))
            .collect();
        self.inner.nodes.store(Arc::new(canonical));
    }

    pub fn all_nodes(&self) -> HashMap<IpAddr, NoiseNode> {
        self.inner.nodes.load().as_ref().clone()
    }
}

#[derive(Clone)]
pub struct NoiseConfig {
    network: NoiseNetworkView,

    pub(crate) local_key: Arc<x25519::KeyPair>,
    pub(crate) pattern: NoisePattern,
    pub(crate) timeout: Duration,

    pub(crate) unsafe_disabled: bool, // allows for nodes to not attempt to do a noise handshake, VERY UNSAFE, FOR DEBUG PURPOSE ONLY
}

impl NoiseConfig {
    pub fn new(
        noise_key: Arc<x25519::KeyPair>,
        network: NoiseNetworkView,
        timeout: Duration,
    ) -> Self {
        NoiseConfig {
            network,
            local_key: noise_key,
            pattern: Default::default(),
            timeout,
            unsafe_disabled: false,
        }
    }

    #[must_use]
    pub fn with_noise_pattern(mut self, pattern: NoisePattern) -> Self {
        self.pattern = pattern;
        self
    }

    #[must_use]
    pub fn with_unsafe_disabled(mut self, disabled: bool) -> Self {
        self.unsafe_disabled = disabled;
        self
    }

    /// Look up the noise key for a specific remote socket address.
    ///
    /// Used on the **initiator** path where we need the responder's public key
    /// to start the handshake. For nym-nodes the port is ignored (one key per IP);
    /// for network monitor agents, the port disambiguates which agent's key to use.
    pub(crate) fn get_noise_key(&self, address: SocketAddr) -> Option<VersionedNoiseKeyV1> {
        let ip_to_check = address.ip().to_canonical();
        let nodes = self.network.inner.nodes.load();

        // Resolve the noise key for `address` from a loaded snapshot of the node map.
        // For [`NoiseNode::NymNode`] entries the port is irrelevant — only the IP is matched.
        // For [`NoiseNode::NetworkMonitorAgent`] entries the port selects the specific agent.
        match nodes.get(&ip_to_check)? {
            NoiseNode::NymNode { key } => Some(*key),
            NoiseNode::NetworkMonitorAgent { nodes } => {
                let port = address.port();
                nodes.iter().find(|n| n.port == port).map(|n| n.key)
            }
        }
    }

    /// Check whether a remote IP is known to support noise.
    /// Used on the responder path where we don't need the remote's key
    /// (the initiator sends it during the handshake).
    // note: in the case of network monitor agents, it must hold
    // that ALL agents on given host support it (or don't support it)
    pub(crate) fn supports_noise(&self, ip_addr: IpAddr) -> bool {
        self.network
            .inner
            .nodes
            .load()
            .contains_key(&ip_addr.to_canonical())
    }
}

#[cfg(test)]
mod tests {
    use snow::params::NoiseParams;

    use super::NoisePattern;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    // The goal of these is to make sure every NoisePatterns are correct and unwrap can be used on them

    #[test]
    fn noise_patterns_are_valid() {
        for pattern in NoisePattern::iter() {
            assert!(NoiseParams::from_str(pattern.as_str()).is_ok())
        }
    }

    #[test]
    fn noise_patterns_psk_position_is_valid() {
        for pattern in NoisePattern::iter() {
            match pattern {
                NoisePattern::XKpsk3 => assert_eq!(pattern.psk_position(), 3),
                NoisePattern::IKpsk2 => assert_eq!(pattern.psk_position(), 2),
            }
        }
    }

    mod noise_key_lookup {
        use super::super::*;
        use nym_crypto::asymmetric::x25519;
        use nym_noise_keys::NoiseVersion;
        use nym_test_utils::helpers::deterministic_rng;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        use std::sync::Arc;
        use std::time::Duration;

        fn dummy_key(seed: u8) -> VersionedNoiseKeyV1 {
            VersionedNoiseKeyV1 {
                supported_version: NoiseVersion::V1,
                x25519_pubkey: x25519::PublicKey::from([seed; 32]),
            }
        }

        fn make_config(nodes: HashMap<IpAddr, NoiseNode>) -> NoiseConfig {
            NoiseConfig::new(
                Arc::new(x25519::KeyPair::new(&mut deterministic_rng())),
                NoiseNetworkView::new(nodes),
                Duration::from_secs(5),
            )
        }

        // -- get_noise_key tests --

        #[test]
        fn nym_node_key_returned_regardless_of_port() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let key = dummy_key(1);
            let config = make_config(HashMap::from([(ip, NoiseNode::new_nym_node(key))]));

            // any port should resolve to the same key
            assert_eq!(config.get_noise_key(SocketAddr::new(ip, 1000)), Some(key));
            assert_eq!(config.get_noise_key(SocketAddr::new(ip, 9999)), Some(key));
        }

        #[test]
        fn agent_key_resolved_by_port() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let key_a = dummy_key(1);
            let key_b = dummy_key(2);

            let node = NoiseNode::NetworkMonitorAgent {
                nodes: vec![
                    NetworkMonitorAgentNode {
                        port: 1000,
                        key: key_a,
                    },
                    NetworkMonitorAgentNode {
                        port: 2000,
                        key: key_b,
                    },
                ],
            };
            let config = make_config(HashMap::from([(ip, node)]));

            assert_eq!(config.get_noise_key(SocketAddr::new(ip, 1000)), Some(key_a));
            assert_eq!(config.get_noise_key(SocketAddr::new(ip, 2000)), Some(key_b));
        }

        #[test]
        fn agent_unknown_port_returns_none() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let node = NoiseNode::NetworkMonitorAgent {
                nodes: vec![NetworkMonitorAgentNode {
                    port: 1000,
                    key: dummy_key(1),
                }],
            };
            let config = make_config(HashMap::from([(ip, node)]));

            assert!(config.get_noise_key(SocketAddr::new(ip, 9999)).is_none());
        }

        #[test]
        fn completely_unknown_address_returns_none() {
            let config = make_config(HashMap::new());

            assert!(config
                .get_noise_key(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 80))
                .is_none());
        }

        #[test]
        fn canonical_ipv6_fallback_for_nym_node() {
            // register under the plain IPv4 address
            let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
            let key = dummy_key(1);
            let config = make_config(HashMap::from([(v4, NoiseNode::new_nym_node(key))]));

            // query with the IPv4-mapped IPv6 form (::ffff:1.2.3.4)
            let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());
            assert_eq!(
                config.get_noise_key(SocketAddr::new(v6_mapped, 1789)),
                Some(key)
            );
        }

        #[test]
        fn canonical_ipv6_fallback_for_agent() {
            let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
            let key = dummy_key(1);
            let node = NoiseNode::NetworkMonitorAgent {
                nodes: vec![NetworkMonitorAgentNode { port: 1000, key }],
            };
            let config = make_config(HashMap::from([(v4, node)]));

            let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());
            assert_eq!(
                config.get_noise_key(SocketAddr::new(v6_mapped, 1000)),
                Some(key)
            );
            // wrong port still returns None even with the fallback
            assert!(config
                .get_noise_key(SocketAddr::new(v6_mapped, 9999))
                .is_none());
        }

        // -- supports_noise tests --

        #[test]
        fn supports_noise_true_for_nym_node() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let config = make_config(HashMap::from([(ip, NoiseNode::new_nym_node(dummy_key(1)))]));

            assert!(config.supports_noise(ip));
        }

        #[test]
        fn supports_noise_true_for_agent_ip() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let node = NoiseNode::NetworkMonitorAgent {
                nodes: vec![NetworkMonitorAgentNode {
                    port: 1000,
                    key: dummy_key(1),
                }],
            };
            let config = make_config(HashMap::from([(ip, node)]));

            assert!(config.supports_noise(ip));
        }

        #[test]
        fn supports_noise_false_for_unknown_ip() {
            let config = make_config(HashMap::new());

            assert!(!config.supports_noise(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
        }

        #[test]
        fn supports_noise_canonical_ipv6_fallback() {
            let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
            let config = make_config(HashMap::from([(v4, NoiseNode::new_nym_node(dummy_key(1)))]));

            let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());
            assert!(config.supports_noise(v6_mapped));
        }

        // -- new_with_agents test --

        #[test]
        fn new_with_agents_builds_correct_view() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let key_a = dummy_key(1);
            let key_b = dummy_key(2);

            let agents = HashMap::from([(
                ip,
                vec![
                    NetworkMonitorAgentNode {
                        port: 1000,
                        key: key_a,
                    },
                    NetworkMonitorAgentNode {
                        port: 2000,
                        key: key_b,
                    },
                ],
            )]);

            let config = NoiseConfig::new(
                Arc::new(x25519::KeyPair::new(&mut deterministic_rng())),
                NoiseNetworkView::new_with_agents(agents),
                Duration::from_secs(5),
            );

            assert_eq!(config.get_noise_key(SocketAddr::new(ip, 1000)), Some(key_a));
            assert_eq!(config.get_noise_key(SocketAddr::new(ip, 2000)), Some(key_b));
            assert!(config.supports_noise(ip));
        }

        // -- swap_view canonicalisation test --

        // Regression: an agent registered via blockchain events flows through `swap_view` (called
        // from `NetworkMonitorAgentsModule::new_agent` and from the periodic network refresher).
        // If a non-canonical (IPv4-mapped IPv6) key reaches `swap_view`, lookups via
        // `supports_noise` (which canonicalises) used to miss, producing the
        // "can't speak Noise yet, falling back to TCP" warning despite the agent being correctly
        // authorised in the routing filter.
        #[tokio::test]
        async fn swap_view_canonicalises_non_canonical_keys() {
            let view = NoiseNetworkView::new_empty();
            let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
            let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());

            let mut nodes = HashMap::new();
            // intentionally insert under the IPv4-mapped form — what a buggy caller might do
            nodes.insert(
                v6_mapped,
                NoiseNode::NetworkMonitorAgent {
                    nodes: vec![NetworkMonitorAgentNode {
                        port: 1000,
                        key: dummy_key(1),
                    }],
                },
            );

            let permit = view.get_update_permit().await;
            view.swap_view(permit, nodes);

            let config = NoiseConfig::new(
                Arc::new(x25519::KeyPair::new(&mut deterministic_rng())),
                view,
                Duration::from_secs(5),
            );

            // lookup via either form must succeed
            assert!(config.supports_noise(v4));
            assert!(config.supports_noise(v6_mapped));
            assert!(config
                .get_noise_key(SocketAddr::new(v6_mapped, 1000))
                .is_some());
        }
    }
}
