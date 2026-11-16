// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The network as an LP route sees it.
//!
//! A hop has to be named by the address the packet is actually sent to, and LP and the legacy
//! mixnet listen on different ones. Rather than hold both on one node and leave every call site to
//! pick, this is a separate set of types: an [`LpRoutingNode`] carries no `mix_host` at all, so
//! nothing holding one can address a legacy hop. The failure that prevents is silent - an LP frame
//! sent to a mix port reaches a TCP listener that is not reading datagrams, and is dropped without
//! a trace.
//!
//! The legacy side in [`crate`] is untouched and knows nothing about this module, so the day LP is
//! the only transport, deleting it is deleting a file.

use crate::node::SupportedRoles;
use crate::rewarded_set::CachedEpochRewardedSet;
use crate::{NymTopologyError, NymTopologyMetadata};
use nym_api_requests::models::described::type_translation::MalformedLPData;
use nym_api_requests::nym_nodes::SkimmedNodeV2;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::peer::LpRemotePeer;
use nym_mixnet_contract_common::NodeId;
use nym_sphinx_addressing::nodes::{NodeIdentity, NymNodeRoutingAddress};
use nym_sphinx_types::Node as SphinxNode;
use rand::prelude::IteratorRandom;
use rand::{CryptoRng, Rng, RngExt};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace};

#[derive(Error, Debug)]
pub enum LpRoutingNodeError {
    #[error("node {node_id} ('{identity}') has not provided any valid ip addresses")]
    NoIpAddressesProvided { node_id: NodeId, identity: String },

    #[error("node {node_id} published '{version}' as its build version, which is not a semver")]
    MalformedBuildVersion { node_id: NodeId, version: String },

    #[error("node {node_id} LP information signature failed to verify")]
    LpSignatureVerificationFailed { node_id: NodeId },

    #[error(transparent)]
    MalformedLpData(#[from] MalformedLPData),
}

/// A node as a hop on an LP route.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LpRoutingNode {
    pub node_id: NodeId,

    /// Where this node's LP control plane listens
    /// Built with the first IP of ip_addresses during creation
    pub control_host: SocketAddr,

    /// Where this node's LP data plane listens: its address on an LP route.
    /// Built with the first IP of ip_addresses during creation
    pub data_host: SocketAddr,

    /// The addresses it announces, allowing clients to choose ipv6 if available
    /// It shouldn't be possible to have an emtpy vec here
    pub ip_addresses: Vec<IpAddr>,

    pub identity_key: ed25519::PublicKey,
    pub sphinx_key: x25519::PublicKey,

    pub supported_roles: SupportedRoles,

    /// LP TCP control address (default: 41264) for establishing LP sessions
    pub control_port: u16,

    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    pub data_port: u16,

    pub remote_peer: LpRemotePeer,

    /// What the node's LP protocol version and KKT ciphersuite are inferred from, since it
    /// advertises neither directly.
    pub build_version: semver::Version,
}

impl Display for LpRoutingNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LP node {} at {}", self.node_id, self.data_host)
    }
}

impl<'a> From<&'a LpRoutingNode> for SphinxNode {
    fn from(value: &'a LpRoutingNode) -> Self {
        // SAFETY: this conversion is infallible as all versions of socket addresses have
        // sufficiently small bytes representation to fit inside `NodeAddressBytes`
        #[allow(clippy::unwrap_used)]
        let node_address_bytes = NymNodeRoutingAddress::from(value.data_host)
            .try_into()
            .unwrap();

        SphinxNode::new(node_address_bytes, value.sphinx_key.into())
    }
}

impl<'a> TryFrom<&'a SkimmedNodeV2> for LpRoutingNode {
    type Error = LpRoutingNodeError;

    fn try_from(value: &'a SkimmedNodeV2) -> Result<Self, Self::Error> {
        let Some(first_ip) = value.ip_addresses.first() else {
            return Err(LpRoutingNodeError::NoIpAddressesProvided {
                node_id: value.node_id,
                identity: value.ed25519_identity_pubkey.to_base58_string(),
            });
        };

        // both the ciphersuite and the LP protocol version are inferred from this, so a node that
        // published something unparseable cannot be talked to at all
        let build_version =
            value
                .build_version
                .parse()
                .map_err(|_| LpRoutingNodeError::MalformedBuildVersion {
                    node_id: value.node_id,
                    version: value.build_version.clone(),
                })?;

        if !value.lp.verify(&value.ed25519_identity_pubkey) {
            return Err(LpRoutingNodeError::LpSignatureVerificationFailed {
                node_id: value.node_id,
            });
        }
        let lp_details = value.lp.content.clone();
        let lp_remote_peer =
            LpRemotePeer::new(lp_details.x25519).with_key_digests(lp_details.kem_keys()?);

        Ok(LpRoutingNode {
            node_id: value.node_id,
            data_host: SocketAddr::new(*first_ip, lp_details.data_port),
            control_host: SocketAddr::new(*first_ip, lp_details.control_port),
            ip_addresses: value.ip_addresses.clone(),
            identity_key: value.ed25519_identity_pubkey,
            sphinx_key: value.x25519_sphinx_pubkey,
            supported_roles: value.supported_roles.into(),
            remote_peer: lp_remote_peer,
            control_port: lp_details.control_port,
            data_port: lp_details.data_port,
            build_version,
        })
    }
}

/// The LP network: the nodes reachable over LP, and the rewarded set restricted to them.
///
/// The twin of [`crate::NymTopology`], and deliberately not a view onto it. Its rewarded set holds
/// only ids this topology can resolve, so hop selection never silently skips a node it cannot
/// reach; a layer with no LP nodes left fails as [`NymTopologyError::NoMixnodesAvailable`] instead
/// of quietly narrowing every route onto whatever subset speaks LP.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LpTopology {
    metadata: NymTopologyMetadata,

    rewarded_set: CachedEpochRewardedSet,

    node_details: HashMap<NodeId, LpRoutingNode>,
}

impl LpTopology {
    /// `rewarded_set` is the network's, and is narrowed here to the nodes actually present.
    pub fn new(
        metadata: NymTopologyMetadata,
        rewarded_set: impl Into<CachedEpochRewardedSet>,
        node_details: Vec<LpRoutingNode>,
    ) -> Self {
        let node_details: HashMap<_, _> =
            node_details.into_iter().map(|n| (n.node_id, n)).collect();
        // Temporary filtering, while we cannot guarantee all active nodes supports LP
        let rewarded_set = restrict_to_known(rewarded_set.into(), &node_details);

        LpTopology {
            metadata,
            rewarded_set,
            node_details,
        }
    }

    /// Everything that converts, with the rest dropped and logged.
    ///
    /// A node that fails to convert has no address on an LP route, so leaving it out is the whole
    /// job: it is absent from the node map and therefore from the rewarded set this narrows.
    pub fn from_nodes<N>(
        metadata: NymTopologyMetadata,
        rewarded_set: impl Into<CachedEpochRewardedSet>,
        nodes: impl Iterator<Item = N>,
    ) -> Self
    where
        N: TryInto<LpRoutingNode>,
        <N as TryInto<LpRoutingNode>>::Error: Display,
    {
        let node_details = nodes
            .filter_map(|node| match node.try_into() {
                Ok(node) => Some(node),
                Err(err) => {
                    debug!("node has no place on an LP route: {err}");
                    None
                }
            })
            .collect();

        Self::new(metadata, rewarded_set, node_details)
    }

    pub fn from_skimmed_nodes(
        metadata: NymTopologyMetadata,
        rewarded_set: impl Into<CachedEpochRewardedSet>,
        nodes: &[SkimmedNodeV2],
    ) -> Self {
        Self::from_nodes(metadata, rewarded_set, nodes.iter())
    }

    /// Add this node's own entry, for a topology that has to contain the node building it.
    pub fn insert_node_details(&mut self, node: LpRoutingNode) {
        self.node_details.insert(node.node_id, node);
    }

    pub fn has_node(&self, identity: ed25519::PublicKey) -> bool {
        self.find_node_by_identity(identity).is_some()
    }

    /// Put `node_id` in the entry-gateway set whatever the epoch says.
    ///
    /// For a node routing through its own topology: it is its own entry, and nothing in the
    /// rewarded set says so.
    pub fn force_set_entry_gateway(&mut self, node_id: NodeId) {
        self.rewarded_set.entry_gateways.insert(node_id);
    }

    pub fn metadata(&self) -> NymTopologyMetadata {
        self.metadata
    }

    pub fn rewarded_set(&self) -> &CachedEpochRewardedSet {
        &self.rewarded_set
    }

    pub fn is_empty(&self) -> bool {
        self.rewarded_set.is_empty() || self.node_details.is_empty()
    }

    pub fn ensure_not_empty(&self) -> Result<(), NymTopologyError> {
        if self.is_empty() {
            return Err(NymTopologyError::EmptyNetworkTopology);
        }
        Ok(())
    }

    /// Whether a route can be built through this view at all: a node on each mix layer, and a
    /// gateway to end at.
    ///
    /// Emptiness is not enough to go on. A view can hold plenty of nodes and still have nothing on
    /// layer 2, which is the state a thin LP rollout actually produces - and every route through it
    /// would fail one at a time, at send, rather than once here.
    ///
    /// Only the counts are checked, because [`Self::new`] has already narrowed the rewarded set to
    /// nodes this view holds: a layer that is non-empty here is a layer that resolves.
    pub fn is_minimally_routable(&self) -> bool {
        let has_layer1 = !self.rewarded_set.layer1.is_empty();
        let has_layer2 = !self.rewarded_set.layer2.is_empty();
        let has_layer3 = !self.rewarded_set.layer3.is_empty();
        let has_entry_gateways = !self.rewarded_set.entry_gateways.is_empty();
        let has_exit_gateways = !self.rewarded_set.exit_gateways.is_empty();

        trace!(
            has_layer1 = %has_layer1,
            has_layer2 = %has_layer2,
            has_layer3 = %has_layer3,
            has_entry_gateways = %has_entry_gateways,
            has_exit_gateways = %has_exit_gateways,
            "LP network status"
        );

        has_layer1 && has_layer2 && has_layer3 && (has_exit_gateways || has_entry_gateways)
    }

    pub fn ensure_minimally_routable(&self) -> Result<(), NymTopologyError> {
        if !self.is_minimally_routable() {
            return Err(NymTopologyError::InsufficientMixingNodes);
        }
        Ok(())
    }

    pub fn find_node(&self, node_id: NodeId) -> Option<&LpRoutingNode> {
        self.node_details.get(&node_id)
    }

    pub fn find_node_by_identity(&self, node_identity: NodeIdentity) -> Option<&LpRoutingNode> {
        self.node_details
            .values()
            .find(|n| n.identity_key == node_identity)
    }

    pub fn egress_by_identity(
        &self,
        node_identity: NodeIdentity,
    ) -> Result<&LpRoutingNode, NymTopologyError> {
        let Some(node) = self.find_node_by_identity(node_identity) else {
            return Err(NymTopologyError::NonExistentNode {
                node_identity: Box::new(node_identity),
            });
        };

        // a 'valid' egress is one that is currently **not** acting as a mixnode
        if let Some(role) = self.rewarded_set.role(node.node_id)
            && role.is_mixnode()
        {
            return Err(NymTopologyError::InvalidEgressRole {
                node_identity: Box::new(node_identity),
            });
        }

        Ok(node)
    }

    fn find_valid_mix_hop<R>(
        &self,
        rng: &mut R,
        id_choices: Vec<NodeId>,
    ) -> Result<&LpRoutingNode, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let mut id_choices = id_choices;
        while !id_choices.is_empty() {
            let index = rng.random_range(0..id_choices.len());

            // SAFETY: this is not run if the vector is empty
            let candidate_id = id_choices[index];
            match self.node_details.get(&candidate_id) {
                Some(node) => return Ok(node),
                None => {
                    id_choices.remove(index);
                    continue;
                }
            }
        }

        Err(NymTopologyError::NoMixnodesAvailable)
    }

    fn choose_mixing_node<R>(
        &self,
        rng: &mut R,
        assigned_nodes: &HashSet<NodeId>,
    ) -> Result<&LpRoutingNode, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let Some(candidate) = assigned_nodes.iter().choose(rng) else {
            return Err(NymTopologyError::NoMixnodesAvailable);
        };

        match self.node_details.get(candidate) {
            Some(node) => Ok(node),
            None => {
                let remaining_choices = assigned_nodes
                    .iter()
                    .filter(|&n| n != candidate)
                    .copied()
                    .collect();
                self.find_valid_mix_hop(rng, remaining_choices)
            }
        }
    }

    fn random_mix_path_nodes<R>(&self, rng: &mut R) -> Result<Vec<&LpRoutingNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        if self.rewarded_set.is_empty() || self.node_details.is_empty() {
            return Err(NymTopologyError::EmptyNetworkTopology);
        }

        // one spare for the egress this is about to have pushed onto it
        let mut mix_route = Vec::with_capacity(4);

        mix_route.push(self.choose_mixing_node(rng, &self.rewarded_set.layer1)?);
        mix_route.push(self.choose_mixing_node(rng, &self.rewarded_set.layer2)?);
        mix_route.push(self.choose_mixing_node(rng, &self.rewarded_set.layer3)?);

        Ok(mix_route)
    }

    /// A whole path: three mix hops and the egress that can reach the recipient.
    ///
    /// The egress is pushed on rather than handed back beside the path, as the legacy twin does -
    /// nothing on this side wants the two apart, and every caller was reassembling them.
    pub fn random_path_to_egress<R>(
        &self,
        rng: &mut R,
        egress_identity: NodeIdentity,
    ) -> Result<Vec<&LpRoutingNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let egress = self.egress_by_identity(egress_identity)?;
        let mut route = self.random_mix_path_nodes(rng)?;
        route.push(egress);
        Ok(route)
    }
}

/// Every rewarded-set id that resolves in `known`, and no others.
///
/// Restricts an LP route to LP nodes
// This is TEMPORARY and reduces the anonymity set by reducing the network size.
// Once all active nodes are supporting LP, this HAS TO be removed
fn restrict_to_known(
    rewarded_set: CachedEpochRewardedSet,
    known: &HashMap<NodeId, LpRoutingNode>,
) -> CachedEpochRewardedSet {
    let retain = |ids: HashSet<NodeId>| -> HashSet<NodeId> {
        ids.into_iter()
            .filter(|id| known.contains_key(id))
            .collect()
    };

    CachedEpochRewardedSet {
        epoch_id: rewarded_set.epoch_id,
        entry_gateways: retain(rewarded_set.entry_gateways),
        exit_gateways: retain(rewarded_set.exit_gateways),
        layer1: retain(rewarded_set.layer1),
        layer2: retain(rewarded_set.layer2),
        layer3: retain(rewarded_set.layer3),
        standby: retain(rewarded_set.standby),
    }
}

/// Supplies a fresh [`LpTopology`], or `None` if it could not be obtained.
///
/// The LP twin of [`TopologyProvider`](crate::provider_trait::TopologyProvider), and here rather
/// than in a client crate for the same reason: a nym-node implements it too, for the mixnet clients
/// and service providers it hosts.
#[cfg_attr(
    all(feature = "provider-trait", not(target_arch = "wasm32")),
    async_trait::async_trait
)]
#[cfg_attr(
    all(feature = "provider-trait", target_arch = "wasm32"),
    async_trait::async_trait(?Send)
)]
pub trait LpTopologyProvider: Send {
    async fn get_new_lp_topology(&mut self) -> Option<LpTopology>;
}

/// The LP twin of [`crate::NymRouteProvider`].
#[derive(Clone, Debug, Default)]
pub struct LpRouteProvider {
    /// The network view routes are constructed against. Shared, so a provider can be cheaply
    /// created for a snapshot of a topology that gets refreshed underneath it.
    pub topology: Arc<LpTopology>,
}

impl From<LpTopology> for LpRouteProvider {
    fn from(topology: LpTopology) -> Self {
        LpRouteProvider::new(Arc::new(topology))
    }
}

impl LpRouteProvider {
    pub fn new(topology: Arc<LpTopology>) -> Self {
        LpRouteProvider { topology }
    }

    pub fn new_empty() -> Self {
        LpTopology::default().into()
    }

    pub fn metadata(&self) -> NymTopologyMetadata {
        self.topology.metadata()
    }

    pub fn current_key_rotation(&self) -> u32 {
        self.topology.metadata().key_rotation_id
    }

    pub fn egress_by_identity(
        &self,
        egress_identity: NodeIdentity,
    ) -> Result<&LpRoutingNode, NymTopologyError> {
        self.topology.egress_by_identity(egress_identity)
    }

    /// A route of nothing but the egress, for a recipient behind the gateway we already reach.
    pub fn empty_path_to_egress(
        &self,
        egress_identity: NodeIdentity,
    ) -> Result<Vec<&LpRoutingNode>, NymTopologyError> {
        Ok(vec![self.egress_by_identity(egress_identity)?])
    }

    /// The twin of [`Self::empty_path_to_egress`] with mix hops in front of that egress, and the
    /// same shape, so a caller choosing between them has nothing to reassemble.
    pub fn random_path_to_egress<R>(
        &self,
        rng: &mut R,
        egress_identity: NodeIdentity,
    ) -> Result<Vec<&LpRoutingNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        self.topology.random_path_to_egress(rng, egress_identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_kkt_ciphersuite::{HashFunction, KEM};
    use std::collections::BTreeMap;

    const KEY: &str = "CBmYewWf43iarBq349KhbfYMc9ys2ebXWd4Vp4CLQ5Rq";

    /// A node with every LP field filled in, including KEM digests.
    fn lp_node(node_id: NodeId) -> LpRoutingNode {
        LpRoutingNode {
            node_id,
            data_host: format!("10.0.0.{node_id}:51264").parse().unwrap(),
            control_host: format!("10.0.0.{node_id}:41264").parse().unwrap(),
            ip_addresses: vec![format!("10.0.0.{node_id}").parse().unwrap()],
            identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK"
                .parse()
                .unwrap(),
            sphinx_key: KEY.parse().unwrap(),
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: true,
                mixnet_exit: true,
            },

            build_version: semver::Version::new(1, 39, 0),
            remote_peer: LpRemotePeer::new(KEY.parse::<x25519::PublicKey>().unwrap().into())
                .with_key_digests(BTreeMap::from([(
                    KEM::MlKem768,
                    BTreeMap::from([(HashFunction::SHA256, vec![0; 32])]),
                )])),
            control_port: 41264,
            data_port: 51264,
        }
    }

    /// A rewarded set naming `ids` on every mix layer.
    fn rewarded_set_over(ids: &[NodeId]) -> CachedEpochRewardedSet {
        let mut set = CachedEpochRewardedSet::default();
        for id in ids {
            set.layer1.insert(*id);
            set.layer2.insert(*id);
            set.layer3.insert(*id);
            set.entry_gateways.insert(*id);
        }
        set
    }

    /// What `--custom-mixnet` loads is exactly what a topology serialises to, LP details included.
    #[test]
    fn a_stored_lp_topology_round_trips() {
        let topology = LpTopology::new(
            NymTopologyMetadata::default(),
            rewarded_set_over(&[4]),
            vec![lp_node(4)],
        );

        let stored = serde_json::to_value(&topology).unwrap();
        let loaded: LpTopology = serde_json::from_value(stored.clone()).unwrap();

        assert_eq!(serde_json::to_value(&loaded).unwrap(), stored);
    }

    /// A node entry without LP details fails to load rather than loading without them: there is no
    /// such thing as an LP node with no LP address.
    #[test]
    fn a_node_without_lp_details_does_not_load() {
        let mut stored = serde_json::to_value(LpTopology::new(
            NymTopologyMetadata::default(),
            rewarded_set_over(&[4]),
            vec![lp_node(4)],
        ))
        .unwrap();

        for field in ["lp_data_host", "build_version"] {
            stored["node_details"]["4"]
                .as_object_mut()
                .unwrap()
                .remove(field);
        }

        assert!(serde_json::from_value::<LpTopology>(stored).is_err());
    }

    /// The rewarded set is narrowed to what this view can actually reach.
    ///
    /// Without this an LP route would silently skip the nodes it cannot resolve and concentrate on
    /// whatever is left, with nothing anywhere saying the network got smaller.
    #[test]
    fn the_rewarded_set_is_narrowed_to_nodes_that_speak_lp() {
        // the set names three nodes; only one of them is in this view
        let topology = LpTopology::new(
            NymTopologyMetadata::default(),
            rewarded_set_over(&[1, 2, 3]),
            vec![lp_node(2)],
        );

        assert_eq!(topology.rewarded_set().layer1, HashSet::from([2]));
        assert_eq!(topology.rewarded_set().layer2, HashSet::from([2]));
        assert_eq!(topology.rewarded_set().layer3, HashSet::from([2]));
        assert_eq!(topology.rewarded_set().entry_gateways, HashSet::from([2]));
    }

    /// A view can be full of nodes and still unroutable, which is why counting them is not enough.
    #[test]
    fn a_populated_view_with_an_empty_layer_is_not_routable() {
        let mut rewarded_set = rewarded_set_over(&[1, 2, 3]);
        // three nodes, all present, but nothing assigned to layer 2
        rewarded_set.layer2.clear();

        let topology = LpTopology::new(
            NymTopologyMetadata::default(),
            rewarded_set,
            vec![lp_node(1), lp_node(2), lp_node(3)],
        );

        assert!(!topology.is_empty());
        assert!(!topology.is_minimally_routable());
        assert!(matches!(
            topology.ensure_minimally_routable().unwrap_err(),
            NymTopologyError::InsufficientMixingNodes
        ));
    }

    /// A network too thin to route says so, rather than building a route through nothing.
    #[test]
    fn a_layer_with_no_lp_nodes_fails_loudly() {
        let mut rewarded_set = rewarded_set_over(&[1]);
        // node 1 speaks LP and sits on layers 1 and 2, but layer 3 names only node 9, which does not
        rewarded_set.layer3 = HashSet::from([9]);

        let topology = LpTopology::new(
            NymTopologyMetadata::default(),
            rewarded_set,
            vec![lp_node(1)],
        );

        let err = topology
            .random_path_to_egress(&mut rand::rng(), lp_node(1).identity_key)
            .unwrap_err();

        assert!(matches!(err, NymTopologyError::NoMixnodesAvailable));
    }

    /// An LP route names hops by the LP data port, never the legacy one.
    ///
    /// The type makes the alternative unrepresentable - there is no `mix_host` here - so this
    /// pins the address that does get used.
    #[test]
    fn a_hop_is_named_by_its_lp_data_port() {
        let node = lp_node(7);
        let hop: SphinxNode = (&node).into();

        let expected: SphinxNode = SphinxNode::new(
            NymNodeRoutingAddress::from(node.data_host)
                .try_into()
                .unwrap(),
            node.sphinx_key.into(),
        );

        assert_eq!(hop.address, expected.address);
    }
}
