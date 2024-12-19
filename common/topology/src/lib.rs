// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ::serde::{Deserialize, Serialize};
use nym_api_requests::nym_nodes::SkimmedNode;
use nym_sphinx_addressing::nodes::NodeIdentity;
use nym_sphinx_types::Node as SphinxNode;
use rand::prelude::IteratorRandom;
use rand::{CryptoRng, Rng};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::net::IpAddr;
use tracing::{debug, warn};

pub use crate::node::{EntryDetails, RoutingNode, SupportedRoles};
pub use error::NymTopologyError;
pub use nym_mixnet_contract_common::nym_node::Role;
pub use nym_mixnet_contract_common::{EpochRewardedSet, NodeId};
pub use rewarded_set::CachedEpochRewardedSet;

pub mod error;
pub mod node;
pub mod rewarded_set;

#[cfg(feature = "provider-trait")]
pub mod provider_trait;
#[cfg(feature = "wasm-serde-types")]
pub mod wasm_helpers;

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

#[derive(Clone, Debug, Default)]
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

    pub fn node_by_identity(&self, node_identity: NodeIdentity) -> Option<&RoutingNode> {
        self.topology.find_node_by_identity(node_identity)
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

    pub fn random_path_to_egress<R>(
        &self,
        rng: &mut R,
        egress_identity: NodeIdentity,
    ) -> Result<(Vec<&RoutingNode>, &RoutingNode), NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        self.topology
            .random_path_to_egress(rng, egress_identity, self.ignore_egress_epoch_roles)
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

    pub fn add_skimmed_nodes(&mut self, nodes: &[SkimmedNode]) {
        self.add_additional_nodes(nodes.iter())
    }

    pub fn add_routing_nodes<B: Borrow<RoutingNode>>(
        &mut self,
        nodes: impl IntoIterator<Item = B>,
    ) {
        for node_details in nodes {
            let node_details = node_details.borrow();
            let node_id = node_details.node_id;
            if self
                .node_details
                .insert(node_id, node_details.clone())
                .is_some()
            {
                debug!("overwriting node details for node {node_id}")
            }
        }
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

    pub fn rewarded_set(&self) -> &CachedEpochRewardedSet {
        &self.rewarded_set
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
        let has_layer1 = self.node_details_exists(&self.rewarded_set.layer1);
        let has_layer2 = self.node_details_exists(&self.rewarded_set.layer2);
        let has_layer3 = self.node_details_exists(&self.rewarded_set.layer3);
        let has_exit_gateways = !self.rewarded_set.exit_gateways.is_empty();
        let has_entry_gateways = !self.rewarded_set.entry_gateways.is_empty();

        debug!(
            has_layer1 = %has_layer1,
            has_layer2 = %has_layer2,
            has_layer3 = %has_layer3,
            has_entry_gateways = %has_entry_gateways,
            has_exit_gateways = %has_exit_gateways,
            "network status"
        );

        has_layer1 && has_layer2 && has_layer3 && (has_exit_gateways || has_entry_gateways)
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

    fn find_valid_mix_hop<R>(
        &self,
        rng: &mut R,
        id_choices: Vec<NodeId>,
    ) -> Result<&RoutingNode, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let mut id_choices = id_choices;
        while !id_choices.is_empty() {
            let index = rng.gen_range(0..id_choices.len());

            // SAFETY: this is not run if the vector is empty
            let candidate_id = id_choices[index];
            match self.node_details.get(&candidate_id) {
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
    ) -> Result<&RoutingNode, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        // try first choice without cloning the ids (because I reckon, more often than not, it will actually work)
        // HashSet's iterator implements `ExactSizeIterator` so choosing **one**  random element
        // is actually not that expensive
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
            return Err(NymTopologyError::NonExistentNode {
                node_identity: Box::new(node_identity),
            });
        };

        // a 'valid' egress is one assigned to either entry role (i.e. entry for another client)
        // or exit role (as a service provider)
        if !ignore_epoch_roles {
            let Some(role) = self.rewarded_set.role(node.node_id) else {
                return Err(NymTopologyError::InvalidEgressRole {
                    node_identity: Box::new(node_identity),
                });
            };
            if !matches!(role, Role::EntryGateway | Role::ExitGateway) {
                return Err(NymTopologyError::InvalidEgressRole {
                    node_identity: Box::new(node_identity),
                });
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

    fn random_mix_path_nodes<R>(&self, rng: &mut R) -> Result<Vec<&RoutingNode>, NymTopologyError>
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

    pub fn random_mix_route<R>(&self, rng: &mut R) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        Ok(self
            .random_mix_path_nodes(rng)?
            .into_iter()
            .map(Into::into)
            .collect())
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

    pub fn random_path_to_egress<R>(
        &self,
        rng: &mut R,
        egress_identity: NodeIdentity,
        ignore_epoch_roles: bool,
    ) -> Result<(Vec<&RoutingNode>, &RoutingNode), NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let egress = self.egress_by_identity(egress_identity, ignore_epoch_roles)?;
        let mix_route = self.random_mix_path_nodes(rng)?;
        Ok((mix_route, egress))
    }

    pub fn nodes_with_role(&self, role: Role) -> impl Iterator<Item = &'_ RoutingNode> {
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
