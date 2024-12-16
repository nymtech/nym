// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ::serde::{Deserialize, Serialize};
use log::{debug, warn};
use nym_sphinx_addressing::nodes::NodeIdentity;
use nym_sphinx_types::Node as SphinxNode;
use rand::prelude::IteratorRandom;
use rand::{CryptoRng, Rng};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::net::IpAddr;

pub use crate::node::{EntryDetails, RoutingNode, SupportedRoles};
pub use error::NymTopologyError;
pub use nym_mixnet_contract_common::nym_node::Role;
pub use nym_mixnet_contract_common::{EpochRewardedSet, NodeId};
pub use rewarded_set::CachedEpochRewardedSet;

pub mod error;

// #[deprecated]
// pub mod gateway;
//
// #[deprecated]
// pub mod mix;
pub mod node;
pub mod rewarded_set;

#[cfg(feature = "provider-trait")]
pub mod provider_trait;
#[cfg(feature = "wasm-serde-types")]
pub(crate) mod wasm_helpers;

#[cfg(feature = "provider-trait")]
pub use provider_trait::{HardcodedTopologyProvider, TopologyProvider};

#[deprecated]
#[derive(Debug, Clone)]
pub enum NetworkAddress {
    IpAddr(IpAddr),
    Hostname(String),
}

#[allow(deprecated)]
mod deprecated_network_address_impls {
    use crate::NetworkAddress;
    use std::convert::Infallible;
    use std::fmt::{Display, Formatter};
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::str::FromStr;
    use std::{fmt, io};

    impl NetworkAddress {
        pub fn as_hostname(self) -> Option<String> {
            match self {
                NetworkAddress::IpAddr(_) => None,
                NetworkAddress::Hostname(s) => Some(s),
            }
        }
    }

    impl NetworkAddress {
        pub fn to_socket_addrs(&self, port: u16) -> io::Result<Vec<SocketAddr>> {
            match self {
                NetworkAddress::IpAddr(addr) => Ok(vec![SocketAddr::new(*addr, port)]),
                NetworkAddress::Hostname(hostname) => {
                    Ok((hostname.as_str(), port).to_socket_addrs()?.collect())
                }
            }
        }
    }

    impl FromStr for NetworkAddress {
        type Err = Infallible;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if let Ok(ip_addr) = s.parse() {
                Ok(NetworkAddress::IpAddr(ip_addr))
            } else {
                Ok(NetworkAddress::Hostname(s.to_string()))
            }
        }
    }

    impl Display for NetworkAddress {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                NetworkAddress::IpAddr(ip_addr) => ip_addr.fmt(f),
                NetworkAddress::Hostname(hostname) => hostname.fmt(f),
            }
        }
    }
}

pub type MixLayer = u8;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NymTopology {
    // for the purposes of future VRF, everyone will need the same view of the network, regardless of performance filtering
    // so we use the same 'master' rewarded set information for that
    //
    // how do we solve the problem of "we have to go through a node that we want to filter out?"
    // ¯\_(ツ)_/¯ that's a future problem
    rewarded_set: CachedEpochRewardedSet,

    node_details: HashMap<NodeId, RoutingNode>,
}

#[derive(Clone, Debug)]
pub struct NymRouteProvider {
    pub topology: NymTopology,

    /// Allow constructing routes with final hop at nodes that are not entry/exit gateways in the current epoch
    pub ignore_egress_epoch_roles: bool,
}

impl From<NymTopology> for NymRouteProvider {
    fn from(topology: NymTopology) -> Self {
        NymRouteProvider {
            topology,
            ignore_egress_epoch_roles: false,
        }
    }
}

impl NymRouteProvider {
    pub fn new(topology: NymTopology, ignore_egress_epoch_roles: bool) -> Self {
        NymRouteProvider {
            topology,
            ignore_egress_epoch_roles,
        }
    }

    pub fn new_empty(ignore_egress_epoch_roles: bool) -> NymRouteProvider {
        let this: Self = NymTopology::default().into();
        this.with_ignore_egress_epoch_roles(ignore_egress_epoch_roles)
    }

    pub fn update(&mut self, new_topology: NymTopology) {
        self.topology = new_topology;
    }

    pub fn clear_topology(&mut self) {
        self.topology = Default::default();
    }

    pub fn with_ignore_egress_epoch_roles(mut self, ignore_egress_epoch_roles: bool) -> Self {
        self.ignore_egress_epoch_roles(ignore_egress_epoch_roles);
        self
    }

    pub fn ignore_egress_epoch_roles(&mut self, ignore_egress_epoch_roles: bool) {
        self.ignore_egress_epoch_roles = ignore_egress_epoch_roles;
    }

    pub fn egress_by_identity(
        &self,
        node_identity: NodeIdentity,
    ) -> Result<&RoutingNode, NymTopologyError> {
        self.topology
            .egress_by_identity(node_identity, self.ignore_egress_epoch_roles)
    }

    /// Tries to create a route to the egress point, such that it goes through mixnode on layer 1,
    /// mixnode on layer2, .... mixnode on layer n and finally the target egress, which can be any known node
    pub fn random_route_to_egress<R>(
        &self,
        rng: &mut R,
        egress_identity: NodeIdentity,
    ) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        self.topology
            .random_route_to_egress(rng, egress_identity, self.ignore_egress_epoch_roles)
    }
}

impl NymTopology {
    pub fn new_empty(rewarded_set: impl Into<CachedEpochRewardedSet>) -> Self {
        NymTopology {
            rewarded_set: rewarded_set.into(),
            node_details: Default::default(),
        }
    }

    pub fn new(
        rewarded_set: impl Into<CachedEpochRewardedSet>,
        node_details: Vec<RoutingNode>,
    ) -> Self {
        NymTopology {
            rewarded_set: rewarded_set.into(),
            node_details: node_details.into_iter().map(|n| (n.node_id, n)).collect(),
        }
    }

    #[cfg(feature = "persistence")]
    pub fn new_from_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        serde_json::from_reader(file).map_err(Into::into)
    }

    pub fn add_additional_nodes<N>(&mut self, nodes: impl Iterator<Item = N>)
    where
        N: TryInto<RoutingNode>,
        <N as TryInto<RoutingNode>>::Error: Display,
    {
        for node in nodes {
            match node.try_into() {
                Ok(node_details) => {
                    let node_id = node_details.node_id;
                    if self.node_details.insert(node_id, node_details).is_some() {
                        debug!("overwriting node details for node {node_id}")
                    }
                }
                Err(err) => {
                    debug!("malformed node details: {err}")
                }
            }
        }
    }

    pub fn has_node_details(&self, node_id: NodeId) -> bool {
        self.node_details.contains_key(&node_id)
    }

    pub fn insert_node_details(&mut self, node_details: RoutingNode) {
        self.node_details.insert(node_details.node_id, node_details);
    }

    pub fn force_set_active(&mut self, node_id: NodeId, role: Role) {
        match role {
            Role::EntryGateway => self.rewarded_set.entry_gateways.insert(node_id),
            Role::Layer1 => self.rewarded_set.layer1.insert(node_id),
            Role::Layer2 => self.rewarded_set.layer2.insert(node_id),
            Role::Layer3 => self.rewarded_set.layer3.insert(node_id),
            Role::ExitGateway => self.rewarded_set.exit_gateways.insert(node_id),
            Role::Standby => self.rewarded_set.standby.insert(node_id),
        };
    }

    fn node_details_exists(&self, ids: &HashSet<NodeId>) -> bool {
        for id in ids {
            if self.node_details.contains_key(id) {
                return true;
            }
        }
        false
    }

    pub fn is_minimally_routable(&self) -> bool {
        self.node_details_exists(&self.rewarded_set.layer1)
            && self.node_details_exists(&self.rewarded_set.layer2)
            && self.node_details_exists(&self.rewarded_set.layer3)
            && (!self.rewarded_set.exit_gateways.is_empty()
                || !self.rewarded_set.entry_gateways.is_empty())

        // TODO: we should also include gateways in that check, but right now we're allowing ALL gateways, even inactive
    }

    pub fn ensure_minimally_routable(&self) -> Result<(), NymTopologyError> {
        if !self.is_minimally_routable() {
            return Err(NymTopologyError::InsufficientMixingNodes);
        }
        Ok(())
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

    fn get_sphinx_node(&self, node_id: NodeId) -> Option<SphinxNode> {
        self.node_details.get(&node_id).map(Into::into)
    }

    fn find_valid_mix_hop<R>(
        &self,
        rng: &mut R,
        id_choices: Vec<NodeId>,
    ) -> Result<SphinxNode, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let mut id_choices = id_choices;
        while !id_choices.is_empty() {
            let index = rng.gen_range(0..id_choices.len());

            // SAFETY: this is not run if the vector is empty
            let candidate_id = id_choices[index];
            match self.get_sphinx_node(candidate_id) {
                Some(node) => {
                    return Ok(node);
                }
                // this will mess with VRF, but that's a future problem
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
    ) -> Result<SphinxNode, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        // try first choice without cloning the ids (because I reckon, more often than not, it will actually work)
        // HashSet's iterator implements `ExactSizeIterator` so choosing **one**  random element
        // is actually not that expensive
        let Some(candidate) = assigned_nodes.iter().choose(rng) else {
            return Err(NymTopologyError::NoMixnodesAvailable);
        };

        match self.get_sphinx_node(*candidate) {
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

    pub fn find_node_by_identity(&self, node_identity: NodeIdentity) -> Option<&RoutingNode> {
        self.node_details
            .values()
            .find(|n| n.identity_key == node_identity)
    }

    pub fn find_node(&self, node_id: NodeId) -> Option<&RoutingNode> {
        self.node_details.get(&node_id)
    }

    pub fn egress_by_identity(
        &self,
        node_identity: NodeIdentity,
        ignore_epoch_roles: bool,
    ) -> Result<&RoutingNode, NymTopologyError> {
        let Some(node) = self.find_node_by_identity(node_identity) else {
            return Err(NymTopologyError::NonExistentNode { node_identity });
        };

        // a 'valid' egress is one assigned to either entry role (i.e. entry for another client)
        // or exit role (as a service provider)
        if !ignore_epoch_roles {
            let Some(role) = self.rewarded_set.role(node.node_id) else {
                return Err(NymTopologyError::InvalidEgressRole { node_identity });
            };
            if !matches!(role, Role::EntryGateway | Role::ExitGateway) {
                return Err(NymTopologyError::InvalidEgressRole { node_identity });
            }
        }
        Ok(node)
    }

    fn egress_node_by_identity(
        &self,
        node_identity: NodeIdentity,
        ignore_epoch_roles: bool,
    ) -> Result<SphinxNode, NymTopologyError> {
        self.egress_by_identity(node_identity, ignore_epoch_roles)
            .map(Into::into)
    }

    pub fn random_mix_route<R>(&self, rng: &mut R) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        if self.rewarded_set.is_empty() || self.node_details.is_empty() {
            return Err(NymTopologyError::EmptyNetworkTopology);
        }

        // we reserve an additional item in the route because we'll have to push an egress
        let mut mix_route = Vec::with_capacity(4);

        mix_route.push(self.choose_mixing_node(rng, &self.rewarded_set.layer1)?);
        mix_route.push(self.choose_mixing_node(rng, &self.rewarded_set.layer2)?);
        mix_route.push(self.choose_mixing_node(rng, &self.rewarded_set.layer3)?);

        Ok(mix_route)
    }

    /// Tries to create a route to the egress point, such that it goes through mixnode on layer 1,
    /// mixnode on layer2, .... mixnode on layer n and finally the target egress, which can be any known node
    pub fn random_route_to_egress<R>(
        &self,
        rng: &mut R,
        egress_identity: NodeIdentity,
        ignore_epoch_roles: bool,
    ) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let egress = self.egress_node_by_identity(egress_identity, ignore_epoch_roles)?;
        let mut mix_route = self.random_mix_route(rng)?;
        mix_route.push(egress);
        Ok(mix_route)
    }

    pub fn nodes_with_role<'a>(&'a self, role: Role) -> impl Iterator<Item = &'a RoutingNode> + 'a {
        self.node_details.values().filter(move |node| match role {
            Role::EntryGateway => self.rewarded_set.entry_gateways.contains(&node.node_id),
            Role::Layer1 => self.rewarded_set.layer1.contains(&node.node_id),
            Role::Layer2 => self.rewarded_set.layer2.contains(&node.node_id),
            Role::Layer3 => self.rewarded_set.layer3.contains(&node.node_id),
            Role::ExitGateway => self.rewarded_set.exit_gateways.contains(&node.node_id),
            Role::Standby => self.rewarded_set.standby.contains(&node.node_id),
        })
    }

    pub fn set_testable_node(&mut self, role: Role, node: impl Into<RoutingNode>) {
        fn init_set(node: NodeId) -> HashSet<NodeId> {
            let mut set = HashSet::new();
            set.insert(node);
            set
        }

        let node = node.into();
        let node_id = node.node_id;
        self.node_details.insert(node.node_id, node);

        match role {
            Role::EntryGateway => self.rewarded_set.entry_gateways = init_set(node_id),
            Role::Layer1 => self.rewarded_set.layer1 = init_set(node_id),
            Role::Layer2 => self.rewarded_set.layer2 = init_set(node_id),
            Role::Layer3 => self.rewarded_set.layer3 = init_set(node_id),
            Role::ExitGateway => self.rewarded_set.exit_gateways = init_set(node_id),
            Role::Standby => {
                warn!("attempting to test node in 'standby' mode - are you sure that's what you meant to do?");
                self.rewarded_set.standby = init_set(node_id)
            }
        }
    }

    pub fn entry_gateways(&self) -> impl Iterator<Item = &RoutingNode> {
        self.node_details
            .values()
            .filter(|n| self.rewarded_set.entry_gateways.contains(&n.node_id))
    }

    // ideally this shouldn't exist...
    pub fn entry_capable_nodes(&self) -> impl Iterator<Item = &RoutingNode> {
        self.node_details
            .values()
            .filter(|n| n.supported_roles.mixnet_entry)
    }

    pub fn mixnodes(&self) -> impl Iterator<Item = &RoutingNode> {
        self.node_details
            .values()
            .filter(|n| self.rewarded_set.is_active_mixnode(&n.node_id))
    }
}

// // the reason for those having `Legacy` prefix is that eventually they should be using
// // exactly the same types
// #[derive(Debug, Clone, Default)]
// pub struct NymTopologyOld {
//     mixes: BTreeMap<MixLayer, Vec<mix::LegacyNode>>,
//     gateways: Vec<gateway::LegacyNode>,
// }
//
// impl NymTopologyOld {
//     #[deprecated]
//     pub async fn new_from_env() -> Result<Self, NymTopologyError> {
//         let api_url = std::env::var(NYM_API)?;
//
//         info!("Generating topology from {api_url}");
//
//         let mixnodes = reqwest::get(&format!("{api_url}/v1/unstable/nym-nodes/mixnodes/skimmed",))
//             .await?
//             .json::<CachedNodesResponse<SkimmedNode>>()
//             .await?
//             .nodes
//             .iter()
//             .map(mix::LegacyNode::try_from)
//             .filter(Result::is_ok)
//             .collect::<Result<Vec<_>, _>>()?;
//
//         let gateways = reqwest::get(&format!("{api_url}/v1/unstable/nym-nodes/gateways/skimmed",))
//             .await?
//             .json::<CachedNodesResponse<SkimmedNode>>()
//             .await?
//             .nodes
//             .iter()
//             .map(gateway::LegacyNode::try_from)
//             .filter(Result::is_ok)
//             .collect::<Result<Vec<_>, _>>()?;
//         let topology = Self::new_unordered(mixnodes, gateways);
//         Ok(topology)
//     }
//
//     pub fn new(
//         mixes: BTreeMap<MixLayer, Vec<mix::LegacyNode>>,
//         gateways: Vec<gateway::LegacyNode>,
//     ) -> Self {
//         NymTopologyOld { mixes, gateways }
//     }
//
//     #[deprecated]
//     pub fn new_unordered(
//         unordered_mixes: Vec<mix::LegacyNode>,
//         gateways: Vec<gateway::LegacyNode>,
//     ) -> Self {
//         let mut mixes = BTreeMap::new();
//         for node in unordered_mixes.into_iter() {
//             let layer = node.layer as MixLayer;
//             let layer_entry = mixes.entry(layer).or_insert_with(Vec::new);
//             layer_entry.push(node)
//         }
//
//         NymTopologyOld { mixes, gateways }
//     }
//
//     pub fn from_unordered<MI, GI, M, G>(unordered_mixes: MI, unordered_gateways: GI) -> Self
//     where
//         MI: Iterator<Item = M>,
//         GI: Iterator<Item = G>,
//         G: TryInto<gateway::LegacyNode>,
//         M: TryInto<mix::LegacyNode>,
//         <G as TryInto<gateway::LegacyNode>>::Error: Display,
//         <M as TryInto<mix::LegacyNode>>::Error: Display,
//     {
//         let mut mixes = BTreeMap::new();
//         let mut gateways = Vec::new();
//
//         for node in unordered_mixes.into_iter() {
//             match node.try_into() {
//                 Ok(mixnode) => mixes
//                     .entry(mixnode.layer as MixLayer)
//                     .or_insert_with(Vec::new)
//                     .push(mixnode),
//                 Err(err) => debug!("malformed mixnode: {err}"),
//             }
//         }
//
//         for node in unordered_gateways.into_iter() {
//             match node.try_into() {
//                 Ok(gateway) => gateways.push(gateway),
//                 Err(err) => debug!("malformed gateway: {err}"),
//             }
//         }
//
//         NymTopologyOld::new(mixes, gateways)
//     }
//
//     #[cfg(feature = "serde")]
//     pub fn new_from_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
//         todo!()
//         // let file = std::fs::File::open(path)?;
//         // serde_json::from_reader(file).map_err(Into::into)
//     }
//
//     pub fn from_basic(basic_mixes: &[SkimmedNode], basic_gateways: &[SkimmedNode]) -> Self {
//         todo!()
//         // nym_topology_from_basic_info(basic_mixes, basic_gateways)
//     }
//
//     pub fn find_mix(&self, mix_id: NodeId) -> Option<&mix::LegacyNode> {
//         for nodes in self.mixes.values() {
//             for node in nodes {
//                 if node.mix_id == mix_id {
//                     return Some(node);
//                 }
//             }
//         }
//         None
//     }
//
//     pub fn find_mix_by_identity(
//         &self,
//         mixnode_identity: IdentityKeyRef,
//     ) -> Option<&mix::LegacyNode> {
//         for nodes in self.mixes.values() {
//             for node in nodes {
//                 if node.identity_key.to_base58_string() == mixnode_identity {
//                     return Some(node);
//                 }
//             }
//         }
//         None
//     }
//
//     pub fn find_gateway(&self, gateway_identity: IdentityKeyRef) -> Option<&gateway::LegacyNode> {
//         self.gateways
//             .iter()
//             .find(|&gateway| gateway.identity_key.to_base58_string() == gateway_identity)
//     }
//
//     pub fn mixes(&self) -> &BTreeMap<MixLayer, Vec<mix::LegacyNode>> {
//         &self.mixes
//     }
//
//     pub fn num_mixnodes(&self) -> usize {
//         self.mixes.values().map(|m| m.len()).sum()
//     }
//
//     pub fn mixes_as_vec(&self) -> Vec<mix::LegacyNode> {
//         let mut mixes: Vec<mix::LegacyNode> = vec![];
//
//         for layer in self.mixes().values() {
//             mixes.extend(layer.to_owned())
//         }
//         mixes
//     }
//
//     pub fn mixes_in_layer(&self, layer: MixLayer) -> Vec<mix::LegacyNode> {
//         assert!([1, 2, 3].contains(&layer));
//         self.mixes.get(&layer).unwrap().to_owned()
//     }
//
//     pub fn gateways(&self) -> &[gateway::LegacyNode] {
//         &self.gateways
//     }
//
//     pub fn get_gateways(&self) -> Vec<gateway::LegacyNode> {
//         self.gateways.clone()
//     }
//
//     pub fn get_gateway(&self, gateway_identity: &NodeIdentity) -> Option<&gateway::LegacyNode> {
//         self.gateways
//             .iter()
//             .find(|gateway| gateway.identity() == gateway_identity)
//     }
//
//     pub fn gateway_exists(&self, gateway_identity: &NodeIdentity) -> bool {
//         self.get_gateway(gateway_identity).is_some()
//     }
//
//     pub fn insert_gateway(&mut self, gateway: gateway::LegacyNode) {
//         self.gateways.push(gateway)
//     }
//
//     pub fn set_gateways(&mut self, gateways: Vec<gateway::LegacyNode>) {
//         self.gateways = gateways
//     }
//
//     pub fn random_gateway<R>(&self, rng: &mut R) -> Result<&gateway::LegacyNode, NymTopologyError>
//     where
//         R: Rng + CryptoRng,
//     {
//         self.gateways
//             .choose(rng)
//             .ok_or(NymTopologyError::NoGatewaysAvailable)
//     }
//
//     /// Returns a vec of size of `num_mix_hops` of mixnodes, such that each subsequent node is on
//     /// next layer, starting from layer 1
//     pub fn random_mix_route<R>(
//         &self,
//         rng: &mut R,
//         num_mix_hops: u8,
//     ) -> Result<Vec<mix::LegacyNode>, NymTopologyError>
//     where
//         R: Rng + CryptoRng + ?Sized,
//     {
//         if self.mixes.len() < num_mix_hops as usize {
//             return Err(NymTopologyError::InvalidNumberOfHopsError {
//                 available: self.mixes.len(),
//                 requested: num_mix_hops as usize,
//             });
//         }
//         let mut route = Vec::with_capacity(num_mix_hops as usize);
//
//         // there is no "layer 0"
//         for layer in 1..=num_mix_hops {
//             // get all mixes on particular layer
//             let layer_mixes = self
//                 .mixes
//                 .get(&layer)
//                 .ok_or(NymTopologyError::EmptyMixLayer { layer })?;
//
//             // choose a random mix from the above list
//             // this can return a 'None' only if slice is empty
//             let random_mix = layer_mixes
//                 .choose(rng)
//                 .ok_or(NymTopologyError::EmptyMixLayer { layer })?;
//             route.push(random_mix.clone());
//         }
//
//         Ok(route)
//     }
//
//     pub fn random_path_to_egress<R>(
//         &self,
//         rng: &mut R,
//         num_mix_hops: u8,
//         egress_identity: &NodeIdentity,
//     ) -> Result<(Vec<mix::LegacyNode>, gateway::LegacyNode), NymTopologyError>
//     where
//         R: Rng + CryptoRng + ?Sized,
//     {
//         let gateway =
//             self.get_gateway(egress_identity)
//                 .ok_or(NymTopologyError::NonExistentGatewayError {
//                     identity_key: egress_identity.to_base58_string(),
//                 })?;
//
//         let path = self.random_mix_route(rng, num_mix_hops)?;
//
//         Ok((path, gateway.clone()))
//     }
//
//     /// Tries to create a route to the specified gateway, such that it goes through mixnode on layer 1,
//     /// mixnode on layer2, .... mixnode on layer n and finally the target gateway
//     pub fn random_route_to_egress<R>(
//         &self,
//         rng: &mut R,
//         num_mix_hops: u8,
//         egress_identity: &NodeIdentity,
//     ) -> Result<Vec<SphinxNode>, NymTopologyError>
//     where
//         R: Rng + CryptoRng + ?Sized,
//     {
//         let gateway =
//             self.get_gateway(egress_identity)
//                 .ok_or(NymTopologyError::NonExistentGatewayError {
//                     identity_key: egress_identity.to_base58_string(),
//                 })?;
//
//         Ok(self
//             .random_mix_route(rng, num_mix_hops)?
//             .into_iter()
//             .map(|node| SphinxNode::from(&node))
//             .chain(std::iter::once(gateway.into()))
//             .collect())
//     }
//
//     /// Overwrites the existing nodes in the specified layer
//     pub fn set_mixes_in_layer(&mut self, layer: u8, mixes: Vec<mix::LegacyNode>) {
//         self.mixes.insert(layer, mixes);
//     }
//
//     /// Checks if a mixnet path can be constructed using the specified number of hops
//     pub fn ensure_can_construct_path_through(
//         &self,
//         num_mix_hops: u8,
//     ) -> Result<(), NymTopologyError> {
//         let mixnodes = self.mixes();
//         // 1. is it completely empty?
//         if mixnodes.is_empty() && self.gateways().is_empty() {
//             return Err(NymTopologyError::EmptyNetworkTopology);
//         }
//
//         // 2. does it have any mixnode at all?
//         if mixnodes.is_empty() {
//             return Err(NymTopologyError::NoMixnodesAvailable);
//         }
//
//         // 3. does it have any gateways at all?
//         if self.gateways().is_empty() {
//             return Err(NymTopologyError::NoGatewaysAvailable);
//         }
//
//         // 4. does it have a mixnode on each layer?
//         for layer in 1..=num_mix_hops {
//             match mixnodes.get(&layer) {
//                 None => return Err(NymTopologyError::EmptyMixLayer { layer }),
//                 Some(layer_nodes) => {
//                     if layer_nodes.is_empty() {
//                         return Err(NymTopologyError::EmptyMixLayer { layer });
//                     }
//                 }
//             }
//         }
//
//         Ok(())
//     }
//
//     pub fn ensure_even_layer_distribution(
//         &self,
//         lower_threshold: f32,
//         upper_threshold: f32,
//     ) -> Result<(), NymTopologyError> {
//         let mixnodes_count = self.num_mixnodes();
//
//         let layers = self
//             .mixes
//             .iter()
//             .map(|(k, v)| (*k, v.len()))
//             .collect::<Vec<_>>();
//
//         if self.gateways.is_empty() {
//             return Err(NymTopologyError::NoGatewaysAvailable);
//         }
//
//         if layers.is_empty() {
//             return Err(NymTopologyError::NoMixnodesAvailable);
//         }
//
//         let upper_bound = (mixnodes_count as f32 * upper_threshold) as usize;
//         let lower_bound = (mixnodes_count as f32 * lower_threshold) as usize;
//
//         for (layer, nodes) in &layers {
//             if nodes < &lower_bound || nodes > &upper_bound {
//                 return Err(NymTopologyError::UnevenLayerDistribution {
//                     layer: *layer,
//                     nodes: *nodes,
//                     lower_bound,
//                     upper_bound,
//                     total_nodes: mixnodes_count,
//                     layer_distribution: layers,
//                 });
//             }
//         }
//
//         Ok(())
//     }
// }
//
// #[cfg(feature = "serde")]
// impl Serialize for NymTopologyOld {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         todo!()
//         // crate::serde::SerializableNymTopology::from(self.clone()).serialize(serializer)
//     }
// }
//
// #[cfg(feature = "serde")]
// impl<'de> Deserialize<'de> for NymTopologyOld {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         todo!()
//         // let serializable = crate::serde::SerializableNymTopology::deserialize(deserializer)?;
//         // serializable.try_into().map_err(::serde::de::Error::custom)
//     }
// }

// pub fn nym_topology_from_basic_info(
//     basic_mixes: &[SkimmedNode],
//     basic_gateways: &[SkimmedNode],
// ) -> NymTopology {
//     todo!()
//     // let mut mixes = BTreeMap::new();
//     // for mix in basic_mixes {
//     //     let Some(layer) = mix.get_mix_layer() else {
//     //         warn!("node {} doesn't have any assigned mix layer!", mix.node_id);
//     //         continue;
//     //     };
//     //
//     //     let layer_entry = mixes.entry(layer).or_insert_with(Vec::new);
//     //     match mix.try_into() {
//     //         Ok(mix) => layer_entry.push(mix),
//     //         Err(err) => {
//     //             warn!("node (mixnode) {} is malformed: {err}", mix.node_id);
//     //             continue;
//     //         }
//     //     }
//     // }
//     //
//     // let mut gateways = Vec::with_capacity(basic_gateways.len());
//     // for gateway in basic_gateways {
//     //     match gateway.try_into() {
//     //         Ok(gate) => gateways.push(gate),
//     //         Err(err) => {
//     //             warn!("node (gateway) {} is malformed: {err}", gateway.node_id);
//     //             continue;
//     //         }
//     //     }
//     // }
//     //
//     // // NymTopology::new(mixes, gateways)
//     // todo!()
// }

#[cfg(test)]
mod converting_mixes_to_vec {
    use super::*;

    #[cfg(test)]
    mod when_nodes_exist {
        use nym_crypto::asymmetric::{encryption, identity};

        use super::*;
        use nym_mixnet_contract_common::LegacyMixLayer;

        #[test]
        fn returns_a_vec_with_hashmap_values() {
            let node1 = mix::LegacyNode {
                mix_id: 42,
                host: "3.3.3.3".parse().unwrap(),
                mix_host: "3.3.3.3:1789".parse().unwrap(),
                identity_key: identity::PublicKey::from_base58_string(
                    "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "C7cown6dYCLZpLiMFC1PaBmhvLvmJmLDJGeRTbPD45bX",
                )
                .unwrap(),
                layer: LegacyMixLayer::One,
                version: "0.2.0".into(),
            };

            let node2 = mix::LegacyNode { ..node1.clone() };

            let node3 = mix::LegacyNode { ..node1.clone() };

            let mut mixes = BTreeMap::new();
            mixes.insert(1, vec![node1, node2]);
            mixes.insert(2, vec![node3]);

            let topology = NymTopology::new(mixes, vec![]);
            let mixvec = topology.mixes_as_vec();
            assert!(mixvec
                .iter()
                .any(|node| &node.identity_key.to_base58_string()
                    == "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7"));
        }
    }

    #[cfg(test)]
    mod when_no_nodes_exist {
        use super::*;

        #[test]
        fn returns_an_empty_vec() {
            let topology = NymTopology::new(BTreeMap::new(), vec![]);
            let mixvec = topology.mixes_as_vec();
            assert!(mixvec.is_empty());
        }
    }
}
