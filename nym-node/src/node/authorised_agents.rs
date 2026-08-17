// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The node's view of the network monitor agents authorised by the network-monitors contract.
//!
//! The authorised set is loaded in full once at startup (a failed load aborts startup) and is
//! thereafter kept current in real time through nyxd websocket events. Both paths parse an
//! announced entry into an [`AuthorisedAgent`] and fold it through [`AuthorisedAgentsView`], the
//! sole writer to the structures derived from the set, so that key validation and keying
//! discipline each live in exactly one place.

use crate::node::routing_filter::network_filter::RoutableNetworkMonitors;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_noise::config::{NetworkMonitorAgentNode, NoiseNetworkView, NoiseNode};
use nym_noise_keys::{NoiseVersion, VersionedNoiseKeyV1};
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::net::SocketAddr;
use tracing::{debug, error, info, warn};

/// An authorised network monitor agent with its announced keys parsed.
pub(crate) struct AuthorisedAgent {
    pub(crate) mixnet_address: SocketAddr,

    pub(crate) noise_key: VersionedNoiseKeyV1,

    /// The agent's announced ed25519 client identity, if it has a usable one. Absent for an agent
    /// authorised before the field existed; neither absence nor a malformed value costs the agent
    /// its mixnet-path authorisation, it merely cannot be recognised on the client-session path.
    // consumed by the client-session gate, which is added separately
    #[allow(dead_code)]
    pub(crate) ed25519_identity: Option<ed25519::PublicKey>,
}

impl AuthorisedAgent {
    /// Parse the keys an agent announced, as carried by both the startup contract query and the
    /// `AuthoriseNetworkMonitor` blockchain event.
    ///
    /// Returns `None`, having logged the reason, if the noise key is unusable: without it the agent
    /// cannot be registered at all. A malformed identity is dropped rather than rejecting the
    /// agent, since the identity only gates the client-session path.
    pub(crate) fn parse_announced(
        mixnet_address: SocketAddr,
        bs58_x25519_noise: &str,
        noise_version: u8,
        bs58_ed25519_identity: Option<&str>,
    ) -> Option<Self> {
        let Ok(x25519_pubkey) = x25519::PublicKey::from_base58_string(bs58_x25519_noise) else {
            error!(
                "network monitor agent {mixnet_address} has announced an invalid noise key - ignoring"
            );
            return None;
        };

        // the contract validates the identity on shape, so an unusable value here means either a
        // pre-validation entry or a contract that has diverged from this node's expectations
        let ed25519_identity = bs58_ed25519_identity.and_then(|bs58_identity| {
            ed25519::PublicKey::from_base58_string(bs58_identity)
                .inspect_err(|err| {
                    warn!(
                        "network monitor agent {mixnet_address} has announced an invalid ed25519 identity ({err}) - it will not be recognised on the client-session path"
                    )
                })
                .ok()
        });

        Some(AuthorisedAgent {
            mixnet_address,
            noise_key: VersionedNoiseKeyV1 {
                supported_version: NoiseVersion::from(noise_version),
                x25519_pubkey,
            },
            ed25519_identity,
        })
    }

    /// Parse an entry as returned by the contract's paged agent query.
    pub(crate) fn parse_contract_entry(agent: AuthorisedNetworkMonitor) -> Option<Self> {
        Self::parse_announced(
            agent.mixnet_address,
            &agent.bs58_x25519_noise,
            agent.noise_version,
            agent.bs58_ed25519_identity.as_deref(),
        )
    }
}

/// Owns every structure derived from the authorised-agent set and is its sole writer.
///
/// Cloning is cheap: each field is a handle to shared state.
#[derive(Clone)]
pub(crate) struct AuthorisedAgentsView {
    /// Canonical-IP-keyed routing set, gating packet routing and the sphinx replay bypass.
    routing: RoutableNetworkMonitors,

    /// Canonical-IP-keyed noise key map. Shared with the nym-api topology refresher, which owns
    /// its nym-node entries, so agent updates must never disturb those.
    noise_view: NoiseNetworkView,
}

impl AuthorisedAgentsView {
    pub(crate) fn new(routing: RoutableNetworkMonitors, noise_view: NoiseNetworkView) -> Self {
        AuthorisedAgentsView {
            routing,
            noise_view,
        }
    }

    /// Register an authorised agent in the routing set and the noise key map.
    pub(crate) async fn add_agent(&self, agent: AuthorisedAgent) {
        let address = agent.mixnet_address;
        debug!("adding NM agent {address}");

        // canonicalise so lookups via supports_noise (which canonicalises) always match
        let ip = address.ip().to_canonical();
        let port = address.port();

        // add ip to the routing filter (it's a no-op if it already exists)
        self.routing.add_known(ip);

        // add noise key to the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();

        match nodes.get_mut(&ip) {
            None => {
                nodes.insert(ip, NoiseNode::new_agent(address, agent.noise_key));
            }
            Some(existing_entry) => match existing_entry {
                NoiseNode::NymNode { .. } => {
                    error!(
                        "the authorised agent runs on the same host as a known nym-node! ignoring"
                    );
                }
                NoiseNode::NetworkMonitorAgent { nodes } => {
                    if let Some(existing) = nodes.iter_mut().find(|n| n.port == port) {
                        existing.key = agent.noise_key;
                    } else {
                        nodes.push(NetworkMonitorAgentNode {
                            port,
                            key: agent.noise_key,
                        });
                    }
                }
            },
        }

        self.noise_view.swap_view(update_permit, nodes);
    }

    /// Remove a revoked agent.
    ///
    /// The IP's entries survive until its last agent is revoked, since several agents may share a
    /// host, disambiguated by port.
    pub(crate) async fn remove_agent(&self, address: SocketAddr) {
        debug!("revoking NM agent {address}");

        // canonicalise to match the stored representation
        let ip = address.ip().to_canonical();

        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();

        let mut final_agent = false;
        match nodes.get_mut(&ip) {
            None => {
                warn!("attempted to revoke a non-existent agent at {address}");
                return;
            }
            Some(node) => match node {
                NoiseNode::NymNode { .. } => {
                    error!(
                        "the revoked agent runs on the same host as a known nym-node! ignoring the revocation"
                    );
                    return;
                }
                NoiseNode::NetworkMonitorAgent { nodes } => {
                    nodes.retain(|agent| agent.port != address.port());
                    if nodes.is_empty() {
                        final_agent = true;
                    }
                }
            },
        }

        if final_agent {
            nodes.remove(&ip);
            self.routing.remove_known(ip);
        }
        self.noise_view.swap_view(update_permit, nodes);
    }

    /// Remove every authorised agent.
    pub(crate) async fn remove_all(&self) {
        info!("revoking all NM agents");

        self.routing.reset();

        // remove all noise keys from the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();

        // Only remove NM agent entries; nym-node entries must be preserved because they are
        // managed by a completely separate code path (the nym-api topology refresher) and
        // would not be restored until the next full topology refresh cycle.
        nodes.retain(|_, node| node.is_nym_node());
        self.noise_view.swap_view(update_permit, nodes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;
    use std::net::{IpAddr, Ipv4Addr};

    fn view() -> AuthorisedAgentsView {
        AuthorisedAgentsView::new(
            RoutableNetworkMonitors::default(),
            NoiseNetworkView::new_empty(),
        )
    }

    fn bs58_noise_key() -> String {
        x25519::PublicKey::from(&x25519::PrivateKey::new(&mut deterministic_rng()))
            .to_base58_string()
    }

    fn noise_key() -> VersionedNoiseKeyV1 {
        VersionedNoiseKeyV1 {
            supported_version: NoiseVersion::from(1),
            x25519_pubkey: x25519::PublicKey::from(&x25519::PrivateKey::new(
                &mut deterministic_rng(),
            )),
        }
    }

    fn agent(address: SocketAddr) -> AuthorisedAgent {
        AuthorisedAgent {
            mixnet_address: address,
            noise_key: noise_key(),
            ed25519_identity: None,
        }
    }

    fn agent_ports(view: &AuthorisedAgentsView, ip: IpAddr) -> Vec<u16> {
        match view.noise_view.all_nodes().get(&ip) {
            Some(NoiseNode::NetworkMonitorAgent { nodes }) => {
                nodes.iter().map(|n| n.port).collect()
            }
            other => panic!("expected agent entries under {ip}, got: {other:?}"),
        }
    }

    fn address(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), port)
    }

    // An unusable noise key means the agent cannot be registered at all, so parsing must reject it
    // rather than handing the view an entry it would have to validate itself.
    #[test]
    fn parsing_rejects_an_agent_with_an_invalid_noise_key() {
        assert!(
            AuthorisedAgent::parse_announced(address(39322), "not-a-key", 1, None).is_none(),
            "an agent with an unusable noise key must not parse"
        );
    }

    // A missing identity is a validly authorised agent: one authorised before the field existed.
    #[test]
    fn parsing_accepts_an_agent_without_an_identity() {
        let agent = AuthorisedAgent::parse_announced(address(39322), &bs58_noise_key(), 1, None)
            .expect("an agent without an identity must still parse");

        assert!(agent.ed25519_identity.is_none());
    }

    // A malformed identity must not cost the agent its mixnet-path authorisation - it only makes
    // the agent unrecognisable on the client-session path.
    #[test]
    fn parsing_drops_a_malformed_identity_but_keeps_the_agent() {
        let agent = AuthorisedAgent::parse_announced(
            address(39322),
            &bs58_noise_key(),
            1,
            Some("not-an-identity"),
        )
        .expect("a malformed identity must not reject the agent");

        assert!(agent.ed25519_identity.is_none());
    }

    #[test]
    fn parsing_keeps_a_valid_identity() {
        let identity = ed25519::KeyPair::new(&mut deterministic_rng());

        let agent = AuthorisedAgent::parse_announced(
            address(39322),
            &bs58_noise_key(),
            1,
            Some(&identity.public_key().to_base58_string()),
        )
        .expect("a well-formed agent must parse");

        assert_eq!(Some(*identity.public_key()), agent.ed25519_identity);
    }

    // Regression: an agent must end up keyed in the noise map under the **canonical** IP form, so
    // the responder's `supports_noise` (which canonicalises on lookup) finds it regardless of
    // whether the inbound socket presents plain IPv4 or the v4-mapped IPv6 form. Before the fix,
    // the event path inserted `address.ip()` raw, leaving the map keyed on a non-canonical
    // IPv4-mapped IPv6 address whenever the contract submission used that form, while the routing
    // filter (which canonicalises on both sides) accepted the packet — producing the "can't speak
    // Noise yet, falling back to TCP" warning.
    #[tokio::test]
    async fn add_agent_stores_under_canonical_ip() {
        let view = view();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());

        // register agent using v4-mapped IPv6 form (the form that triggered the bug)
        view.add_agent(agent(SocketAddr::new(v6_mapped, 39322)))
            .await;

        let stored = view.noise_view.all_nodes();
        // the stored key must be canonical so canonical-form lookups succeed
        assert!(
            stored.contains_key(&v4),
            "noise map must contain the canonical IPv4 key, got: {:?}",
            stored.keys().collect::<Vec<_>>()
        );
    }

    // Counterpart: same invariant when the contract submission already used plain IPv4 — the
    // map should still be keyed on the canonical form (which for plain IPv4 is itself).
    #[tokio::test]
    async fn add_agent_stores_under_canonical_ip_for_plain_v4_input() {
        let view = view();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

        view.add_agent(agent(SocketAddr::new(v4, 39322))).await;

        assert!(view.noise_view.all_nodes().contains_key(&v4));
    }

    // Two agents sharing a host in *different* address forms must collapse onto the one canonical
    // entry, keeping both ports. The startup load used to assemble its map keyed on the raw
    // announced IP and let the view's constructor canonicalise, so the two forms began as separate
    // keys and one silently overwrote the other on canonicalisation.
    #[tokio::test]
    async fn add_agent_merges_mixed_address_forms_of_one_host() {
        let view = view();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());

        view.add_agent(agent(SocketAddr::new(v4, 39322))).await;
        view.add_agent(agent(SocketAddr::new(v6_mapped, 39323)))
            .await;

        let stored = view.noise_view.all_nodes();
        assert_eq!(1, stored.len());

        let mut ports = agent_ports(&view, v4);
        ports.sort_unstable();
        assert_eq!(vec![39322, 39323], ports);
    }

    // An IP's noise entry and routing entry must both survive until the last agent on that host is
    // revoked, since agents sharing a host are disambiguated only by port.
    #[tokio::test]
    async fn remove_agent_keeps_the_host_until_its_last_agent_is_revoked() {
        let view = view();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

        view.add_agent(agent(address(39322))).await;
        view.add_agent(agent(address(39323))).await;

        view.remove_agent(address(39322)).await;
        assert_eq!(vec![39323], agent_ports(&view, v4));
        assert!(view.routing.is_known(&v4));

        view.remove_agent(address(39323)).await;
        assert!(view.noise_view.all_nodes().is_empty());
        assert!(!view.routing.is_known(&v4));
    }

    // Revoking everything must leave nym-node entries alone: they come from the topology refresher
    // and would not be restored until its next full refresh cycle.
    #[tokio::test]
    async fn remove_all_preserves_nym_node_entries() {
        let view = view();

        let agent_ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let node_ip = IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8));

        view.add_agent(agent(address(39322))).await;

        let permit = view.noise_view.get_update_permit().await;
        let mut nodes = view.noise_view.all_nodes();
        nodes.insert(node_ip, NoiseNode::new_nym_node(noise_key()));
        view.noise_view.swap_view(permit, nodes);

        view.remove_all().await;

        let stored = view.noise_view.all_nodes();
        assert_eq!(1, stored.len());
        assert!(stored[&node_ip].is_nym_node());
        assert!(!view.routing.is_known(&agent_ip));
    }
}
