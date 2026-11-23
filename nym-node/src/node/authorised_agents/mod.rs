// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The node's view of the network monitor agents authorised by the network-monitors contract.
//!
//! The whole authorised set is loaded from the contract at a single block height, at boot and again
//! every time the nyx chain watcher has to be restarted, and is kept current in between by the
//! watcher's contract events. A failed load at boot stops the node from starting unless it runs
//! with `--allow-nyx-watcher-startup-failure`; a failed reload keeps the set already held. Both
//! paths parse an announced entry into an [`AuthorisedAgent`] and go through
//! [`AuthorisedAgentsView`], the sole writer to the structures derived from the set, so that key
//! validation and keying discipline each live in exactly one place.

use crate::node::routing_filter::network_filter::RoutableNetworkMonitors;
use nym_authorised_network_monitors::AuthorisedMonitorIdentities;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_noise::config::NoiseNetworkView;
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
    pub(crate) ed25519_identity: Option<ed25519::PublicKey>,
}

impl AuthorisedAgent {
    /// Parse the keys an agent announced, as carried by both the contract's agent query and the
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

    /// Announced client identities, keyed by identity rather than by address: the one structure
    /// here that the source IP plays no part in.
    identities: AuthorisedMonitorIdentities,
}

impl AuthorisedAgentsView {
    pub(crate) fn new(
        routing: RoutableNetworkMonitors,
        noise_view: NoiseNetworkView,
        identities: AuthorisedMonitorIdentities,
    ) -> Self {
        AuthorisedAgentsView {
            routing,
            noise_view,
            identities,
        }
    }

    /// Register an authorised agent in the routing set and the noise key map.
    pub(crate) async fn add_agent(&self, agent: AuthorisedAgent) {
        let address = agent.mixnet_address;
        debug!("adding NM agent {address}");

        // add ip to the routing filter (it's a no-op if it already exists)
        self.routing.add_known(address.ip());

        // record what this entry announces, which for an agent re-announcing a changed identity
        // also retires the superseded one
        self.identities
            .set_announced(address, agent.ed25519_identity);

        // add noise key to the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
        nodes.add_agent(address, agent.noise_key);
        self.noise_view.swap_view(update_permit, nodes);
    }

    /// Remove a revoked agent.
    ///
    /// The IP's entries survive until its last agent is revoked, since several agents may share a
    /// host, disambiguated by port.
    pub(crate) async fn remove_agent(&self, address: SocketAddr) {
        debug!("revoking NM agent {address}");

        // the identity itself only goes once its last announcing entry has been revoked, since an
        // agent authorises one entry per address family and both carry it
        self.identities.remove(address);

        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
        if nodes.remove_agent(address) {
            self.routing.remove_known(address.ip());
        }
        self.noise_view.swap_view(update_permit, nodes);
    }

    /// Remove every authorised agent.
    pub(crate) async fn remove_all(&self) {
        info!("revoking all NM agents");

        self.routing.reset();
        self.identities.reset();

        // nym-node entries stay: they come from the nym-api topology refresher and would not
        // be restored until its next full refresh cycle
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
        nodes.remove_all_agents();
        self.noise_view.swap_view(update_permit, nodes);
    }

    /// Replace the whole authorised set with `agents`, as loaded from the contract.
    ///
    /// Each structure is built aside and published with a single swap, so none is ever seen empty
    /// or half-built. The noise map's update permit is held throughout.
    pub(crate) async fn replace_all(&self, agents: Vec<AuthorisedAgent>) {
        info!(
            "replacing the authorised NM agent set with {} agent(s)",
            agents.len()
        );
        let update_permit = self.noise_view.get_update_permit().await;

        self.routing
            .replace_known(agents.iter().map(|agent| agent.mixnet_address.ip()));
        self.identities.replace_all(
            agents
                .iter()
                .map(|agent| (agent.mixnet_address, agent.ed25519_identity))
                .collect(),
        );

        // nym-node entries stay: they come from the nym-api topology refresher
        let mut nodes = self.noise_view.all_nodes();
        nodes.remove_all_agents();
        for agent in agents {
            nodes.add_agent(agent.mixnet_address, agent.noise_key);
        }
        self.noise_view.swap_view(update_permit, nodes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_noise::config::NoiseNode;
    use nym_test_utils::helpers::u64_seeded_rng;
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr};

    fn view() -> AuthorisedAgentsView {
        AuthorisedAgentsView::new(
            RoutableNetworkMonitors::default(),
            NoiseNetworkView::new_empty(),
            AuthorisedMonitorIdentities::default(),
        )
    }

    // every key helper is seeded per call: the shared `deterministic_rng` uses one fixed seed, so
    // it would hand every call the same key and quietly pass any test meant to tell two apart
    fn bs58_noise_key(seed: u64) -> String {
        x25519::PublicKey::from(&x25519::PrivateKey::new(&mut u64_seeded_rng(seed)))
            .to_base58_string()
    }

    fn noise_key(seed: u64) -> VersionedNoiseKeyV1 {
        VersionedNoiseKeyV1 {
            supported_version: NoiseVersion::from(1),
            x25519_pubkey: x25519::PublicKey::from(&x25519::PrivateKey::new(&mut u64_seeded_rng(
                seed,
            ))),
        }
    }

    fn identity(seed: u64) -> ed25519::PublicKey {
        *ed25519::KeyPair::new(&mut u64_seeded_rng(seed)).public_key()
    }

    fn agent(address: SocketAddr) -> AuthorisedAgent {
        agent_announcing(address, None)
    }

    fn agent_announcing(
        address: SocketAddr,
        ed25519_identity: Option<ed25519::PublicKey>,
    ) -> AuthorisedAgent {
        AuthorisedAgent {
            mixnet_address: address,
            noise_key: noise_key(address.port() as u64),
            ed25519_identity,
        }
    }

    fn agent_ports(view: &AuthorisedAgentsView, ip: IpAddr) -> Vec<u16> {
        match view.noise_view.all_nodes().get(ip) {
            Some(NoiseNode::NetworkMonitorAgent { nodes }) => {
                nodes.iter().map(|n| n.port).collect()
            }
            other => panic!("expected agent entries under {ip}, got: {other:?}"),
        }
    }

    fn address(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), port)
    }

    fn host(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, n))
    }

    /// An agent alone on `host(n)`, announcing `identity(n)`.
    fn agent_on_host(n: u8) -> AuthorisedAgent {
        agent_announcing(SocketAddr::new(host(n), 39322), Some(identity(n as u64)))
    }

    /// Make `ip` the noise map's only nym-node, as the topology refresher would.
    async fn put_nym_node(view: &AuthorisedAgentsView, ip: IpAddr) {
        let permit = view.noise_view.get_update_permit().await;
        let mut nodes = view.noise_view.all_nodes();
        nodes.replace_nym_nodes(HashMap::from([(ip, noise_key(9))]));
        view.noise_view.swap_view(permit, nodes);
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
        let agent = AuthorisedAgent::parse_announced(address(39322), &bs58_noise_key(1), 1, None)
            .expect("an agent without an identity must still parse");

        assert!(agent.ed25519_identity.is_none());
    }

    // A malformed identity must not cost the agent its mixnet-path authorisation - it only makes
    // the agent unrecognisable on the client-session path.
    #[test]
    fn parsing_drops_a_malformed_identity_but_keeps_the_agent() {
        let agent = AuthorisedAgent::parse_announced(
            address(39322),
            &bs58_noise_key(1),
            1,
            Some("not-an-identity"),
        )
        .expect("a malformed identity must not reject the agent");

        assert!(agent.ed25519_identity.is_none());
    }

    #[test]
    fn parsing_keeps_a_valid_identity() {
        let identity = identity(1);

        let agent = AuthorisedAgent::parse_announced(
            address(39322),
            &bs58_noise_key(2),
            1,
            Some(&identity.to_base58_string()),
        )
        .expect("a well-formed agent must parse");

        assert_eq!(Some(identity), agent.ed25519_identity);
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
            stored.contains(v4),
            "noise map must contain the canonical IPv4 key, got: {stored:?}"
        );
    }

    // Counterpart: same invariant when the contract submission already used plain IPv4 — the
    // map should still be keyed on the canonical form (which for plain IPv4 is itself).
    #[tokio::test]
    async fn add_agent_stores_under_canonical_ip_for_plain_v4_input() {
        let view = view();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

        view.add_agent(agent(SocketAddr::new(v4, 39322))).await;

        assert!(view.noise_view.all_nodes().contains(v4));
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

    // An agent authorises one entry per address family and both carry its identity, so the identity
    // must survive until the last of them is revoked. The two entries reach the node as independent
    // authorisations; nothing tells it they belong to one agent.
    #[tokio::test]
    async fn an_agents_identity_survives_until_both_of_its_entries_are_revoked() {
        let view = view();
        let identity = identity(1);

        let v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 39322);
        let v6 = SocketAddr::new(
            IpAddr::V6(Ipv4Addr::new(5, 6, 7, 8).to_ipv6_mapped()),
            39322,
        );

        view.add_agent(agent_announcing(v4, Some(identity))).await;
        view.add_agent(agent_announcing(v6, Some(identity))).await;

        assert!(view.identities.is_announced(&identity));

        view.remove_agent(v4).await;
        assert!(
            view.identities.is_announced(&identity),
            "the agent's other entry still announces this identity"
        );

        view.remove_agent(v6).await;
        assert!(!view.identities.is_announced(&identity));
    }

    // An agent with no usable identity is validly authorised - it simply cannot be recognised on
    // the client-session path - so it must still reach the mixnet-path structures.
    #[tokio::test]
    async fn an_agent_without_an_identity_is_still_registered() {
        let view = view();

        let announced =
            AuthorisedAgent::parse_announced(address(39322), &bs58_noise_key(1), 1, None)
                .expect("an agent without an identity must parse");

        view.add_agent(announced).await;

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        assert!(view.routing.is_known(&v4));
        assert_eq!(vec![39322], agent_ports(&view, v4));
    }

    // A malformed identity is dropped at the parse boundary, so the same must hold for an agent
    // whose announced identity was unusable: authorised on the mixnet path, unknown on the session
    // path. Otherwise a typo'd key would silently cost an agent its probes.
    #[tokio::test]
    async fn an_agent_with_a_malformed_identity_is_still_registered() {
        let view = view();

        let announced = AuthorisedAgent::parse_announced(
            address(39322),
            &bs58_noise_key(1),
            1,
            Some("not-an-identity"),
        )
        .expect("a malformed identity must not reject the agent");

        view.add_agent(announced).await;

        assert!(
            view.routing
                .is_known(&IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
        );
    }

    // The contract's agent save is an upsert, so an agent that re-announces a changed identity must
    // not leave the superseded one holding an unmetered-session exemption.
    #[tokio::test]
    async fn a_re_announced_identity_retires_the_previous_one() {
        let view = view();
        let old = identity(1);
        let new = identity(2);

        view.add_agent(agent_announcing(address(39322), Some(old)))
            .await;
        view.add_agent(agent_announcing(address(39322), Some(new)))
            .await;

        assert!(!view.identities.is_announced(&old));
        assert!(view.identities.is_announced(&new));
    }

    // Revoking everything must leave nym-node entries alone: they come from the topology refresher
    // and would not be restored until its next full refresh cycle.
    #[tokio::test]
    async fn remove_all_preserves_nym_node_entries() {
        let view = view();
        let identity = identity(1);

        let agent_ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let node_ip = IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8));

        view.add_agent(agent_announcing(address(39322), Some(identity)))
            .await;
        put_nym_node(&view, node_ip).await;

        view.remove_all().await;

        let stored = view.noise_view.all_nodes();
        assert_eq!(1, stored.len());
        assert!(stored.get(node_ip).is_some_and(NoiseNode::is_nym_node));
        assert!(!view.routing.is_known(&agent_ip));
        assert!(!view.identities.is_announced(&identity));
    }

    // A reload has to leave exactly the loaded set: an agent revoked while no watcher was running
    // goes, one authorised meanwhile arrives, and the topology refresher's entries stay.
    #[tokio::test]
    async fn replacing_the_set_drops_what_is_gone_and_adds_what_is_new() {
        let view = view();
        let [a, b, c, d] = [1, 2, 3, 4];
        let node_ip = IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8));

        for n in [a, b, c] {
            view.add_agent(agent_on_host(n)).await;
        }
        put_nym_node(&view, node_ip).await;

        view.replace_all(vec![agent_on_host(a), agent_on_host(b), agent_on_host(d)])
            .await;

        let noise = view.noise_view.all_nodes();
        for n in [a, b, d] {
            assert!(view.routing.is_known(&host(n)));
            assert!(noise.contains(host(n)));
            assert!(view.identities.is_announced(&identity(n as u64)));
        }
        assert!(!view.routing.is_known(&host(c)));
        assert!(!noise.contains(host(c)));
        assert!(!view.identities.is_announced(&identity(c as u64)));
        assert!(noise.get(node_ip).is_some_and(NoiseNode::is_nym_node));
    }
}
