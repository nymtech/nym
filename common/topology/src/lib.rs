// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(unknown_lints)]
// clippy::to_string_trait_impl is not on stable as of 1.77

use crate::filter::VersionFilterable;
pub use error::NymTopologyError;
use log::warn;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::{GatewayBond, IdentityKeyRef, MixId};
use nym_sphinx_addressing::nodes::NodeIdentity;
use nym_sphinx_types::Node as SphinxNode;
use rand::prelude::SliceRandom;
use rand::{CryptoRng, Rng};
use std::collections::BTreeMap;

use std::fmt::{self, Display, Formatter};
use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;

#[cfg(feature = "serializable")]
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use nym_api_requests::models::DescribedGateway;

pub mod error;
pub mod filter;
pub mod gateway;
pub mod mix;
pub mod random_route_provider;

#[cfg(feature = "provider-trait")]
pub mod provider_trait;

#[cfg(feature = "serializable")]
pub(crate) mod serde;

#[cfg(feature = "serializable")]
pub use crate::serde::{
    SerializableGateway, SerializableMixNode, SerializableNymTopology, SerializableTopologyError,
};

#[cfg(feature = "provider-trait")]
pub use provider_trait::{HardcodedTopologyProvider, TopologyProvider};

#[derive(Debug, Default, Clone)]
pub enum NodeVersion {
    Explicit(semver::Version),

    #[default]
    Unknown,
}

// this is only implemented for backwards compatibility so we wouldn't need to change everything at once
// (also I intentionally implemented `ToString` as opposed to `Display`)
#[allow(clippy::to_string_trait_impl)]
impl ToString for NodeVersion {
    fn to_string(&self) -> String {
        match self {
            NodeVersion::Explicit(semver) => semver.to_string(),
            NodeVersion::Unknown => String::new(),
        }
    }
}

// this is also for backwards compat.
impl<'a> From<&'a str> for NodeVersion {
    fn from(value: &'a str) -> Self {
        if let Ok(semver) = value.parse() {
            NodeVersion::Explicit(semver)
        } else {
            NodeVersion::Unknown
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkAddress {
    IpAddr(IpAddr),
    Hostname(String),
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
    type Err = std::io::Error;

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

pub type MixLayer = u8;

#[derive(Debug, Clone)]
pub struct NymTopology {
    mixes: BTreeMap<MixLayer, Vec<mix::Node>>,
    gateways: Vec<gateway::Node>,
}

impl NymTopology {
    pub fn new(mixes: BTreeMap<MixLayer, Vec<mix::Node>>, gateways: Vec<gateway::Node>) -> Self {
        NymTopology { mixes, gateways }
    }

    pub fn new_unordered(unordered_mixes: Vec<mix::Node>, gateways: Vec<gateway::Node>) -> Self {
        let mut mixes = BTreeMap::new();
        for node in unordered_mixes.into_iter() {
            let layer = node.layer as MixLayer;
            let layer_entry = mixes.entry(layer).or_insert_with(Vec::new);
            layer_entry.push(node)
        }

        NymTopology { mixes, gateways }
    }

    #[cfg(feature = "serializable")]
    pub fn new_from_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        serde_json::from_reader(file).map_err(Into::into)
    }

    pub fn from_detailed(
        mix_details: Vec<MixNodeDetails>,
        gateway_bonds: Vec<GatewayBond>,
    ) -> Self {
        nym_topology_from_detailed(mix_details, gateway_bonds)
    }

    pub fn find_mix(&self, mix_id: MixId) -> Option<&mix::Node> {
        for nodes in self.mixes.values() {
            for node in nodes {
                if node.mix_id == mix_id {
                    return Some(node);
                }
            }
        }
        None
    }

    pub fn find_mix_by_identity(&self, mixnode_identity: IdentityKeyRef) -> Option<&mix::Node> {
        for nodes in self.mixes.values() {
            for node in nodes {
                if node.identity_key.to_base58_string() == mixnode_identity {
                    return Some(node);
                }
            }
        }
        None
    }

    pub fn find_gateway(&self, gateway_identity: IdentityKeyRef) -> Option<&gateway::Node> {
        self.gateways
            .iter()
            .find(|&gateway| gateway.identity_key.to_base58_string() == gateway_identity)
    }

    pub fn mixes(&self) -> &BTreeMap<MixLayer, Vec<mix::Node>> {
        &self.mixes
    }

    pub fn num_mixnodes(&self) -> usize {
        self.mixes.values().map(|m| m.len()).sum()
    }

    pub fn mixes_as_vec(&self) -> Vec<mix::Node> {
        let mut mixes: Vec<mix::Node> = vec![];

        for layer in self.mixes().values() {
            mixes.extend(layer.to_owned())
        }
        mixes
    }

    pub fn mixes_in_layer(&self, layer: MixLayer) -> Vec<mix::Node> {
        assert!([1, 2, 3].contains(&layer));
        self.mixes.get(&layer).unwrap().to_owned()
    }

    pub fn gateways(&self) -> &[gateway::Node] {
        &self.gateways
    }

    pub fn get_gateways(&self) -> Vec<gateway::Node> {
        self.gateways.clone()
    }

    pub fn get_gateway(&self, gateway_identity: &NodeIdentity) -> Option<&gateway::Node> {
        self.gateways
            .iter()
            .find(|gateway| gateway.identity() == gateway_identity)
    }

    pub fn gateway_exists(&self, gateway_identity: &NodeIdentity) -> bool {
        self.get_gateway(gateway_identity).is_some()
    }

    pub fn set_gateways(&mut self, gateways: Vec<gateway::Node>) {
        self.gateways = gateways
    }

    pub fn random_gateway<R>(&self, rng: &mut R) -> Result<&gateway::Node, NymTopologyError>
    where
        R: Rng + CryptoRng,
    {
        self.gateways
            .choose(rng)
            .ok_or(NymTopologyError::NoGatewaysAvailable)
    }

    /// Returns a vec of size of `num_mix_hops` of mixnodes, such that each subsequent node is on
    /// next layer, starting from layer 1
    pub fn random_mix_route<R>(
        &self,
        rng: &mut R,
        num_mix_hops: u8,
    ) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        if self.mixes.len() < num_mix_hops as usize {
            return Err(NymTopologyError::InvalidNumberOfHopsError {
                available: self.mixes.len(),
                requested: num_mix_hops as usize,
            });
        }
        let mut route = Vec::with_capacity(num_mix_hops as usize);

        // there is no "layer 0"
        for layer in 1..=num_mix_hops {
            // get all mixes on particular layer
            let layer_mixes = self
                .mixes
                .get(&layer)
                .ok_or(NymTopologyError::EmptyMixLayer { layer })?;

            // choose a random mix from the above list
            // this can return a 'None' only if slice is empty
            let random_mix = layer_mixes
                .choose(rng)
                .ok_or(NymTopologyError::EmptyMixLayer { layer })?;
            route.push(random_mix.into());
        }

        Ok(route)
    }

    /// Tries to create a route to the specified gateway, such that it goes through mixnode on layer 1,
    /// mixnode on layer2, .... mixnode on layer n and finally the target gateway
    pub fn random_route_to_gateway<R>(
        &self,
        rng: &mut R,
        num_mix_hops: u8,
        gateway_identity: &NodeIdentity,
    ) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        R: Rng + CryptoRng + ?Sized,
    {
        let gateway = self.get_gateway(gateway_identity).ok_or(
            NymTopologyError::NonExistentGatewayError {
                identity_key: gateway_identity.to_base58_string(),
            },
        )?;

        Ok(self
            .random_mix_route(rng, num_mix_hops)?
            .into_iter()
            .chain(std::iter::once(gateway.into()))
            .collect())
    }

    /// Overwrites the existing nodes in the specified layer
    pub fn set_mixes_in_layer(&mut self, layer: u8, mixes: Vec<mix::Node>) {
        self.mixes.insert(layer, mixes);
    }

    /// Checks if a mixnet path can be constructed using the specified number of hops
    pub fn ensure_can_construct_path_through(
        &self,
        num_mix_hops: u8,
    ) -> Result<(), NymTopologyError> {
        let mixnodes = self.mixes();
        // 1. is it completely empty?
        if mixnodes.is_empty() && self.gateways().is_empty() {
            return Err(NymTopologyError::EmptyNetworkTopology);
        }

        // 2. does it have any mixnode at all?
        if mixnodes.is_empty() {
            return Err(NymTopologyError::NoMixnodesAvailable);
        }

        // 3. does it have any gateways at all?
        if self.gateways().is_empty() {
            return Err(NymTopologyError::NoGatewaysAvailable);
        }

        // 4. does it have a mixnode on each layer?
        for layer in 1..=num_mix_hops {
            match mixnodes.get(&layer) {
                None => return Err(NymTopologyError::EmptyMixLayer { layer }),
                Some(layer_nodes) => {
                    if layer_nodes.is_empty() {
                        return Err(NymTopologyError::EmptyMixLayer { layer });
                    }
                }
            }
        }

        Ok(())
    }

    pub fn ensure_even_layer_distribution(
        &self,
        lower_threshold: f32,
        upper_threshold: f32,
    ) -> Result<(), NymTopologyError> {
        let mixnodes_count = self.num_mixnodes();

        let layers = self
            .mixes
            .iter()
            .map(|(k, v)| (*k, v.len()))
            .collect::<Vec<_>>();

        if self.gateways.is_empty() {
            return Err(NymTopologyError::NoGatewaysAvailable);
        }

        if layers.is_empty() {
            return Err(NymTopologyError::NoMixnodesAvailable);
        }

        let upper_bound = (mixnodes_count as f32 * upper_threshold) as usize;
        let lower_bound = (mixnodes_count as f32 * lower_threshold) as usize;

        for (layer, nodes) in &layers {
            if nodes < &lower_bound || nodes > &upper_bound {
                return Err(NymTopologyError::UnevenLayerDistribution {
                    layer: *layer,
                    nodes: *nodes,
                    lower_bound,
                    upper_bound,
                    total_nodes: mixnodes_count,
                    layer_distribution: layers,
                });
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn filter_system_version(&self, expected_version: &str) -> Self {
        self.filter_node_versions(expected_version)
    }

    #[must_use]
    pub fn filter_node_versions(&self, expected_mix_version: &str) -> Self {
        NymTopology {
            mixes: self.mixes.filter_by_version(expected_mix_version),
            gateways: self.gateways.clone(),
        }
    }
}

#[cfg(feature = "serializable")]
impl Serialize for NymTopology {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::serde::SerializableNymTopology::from(self.clone()).serialize(serializer)
    }
}

#[cfg(feature = "serializable")]
impl<'de> Deserialize<'de> for NymTopology {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serializable = crate::serde::SerializableNymTopology::deserialize(deserializer)?;
        serializable.try_into().map_err(::serde::de::Error::custom)
    }
}

pub trait IntoGatewayNode: TryInto<gateway::Node>
where
    <Self as TryInto<gateway::Node>>::Error: Display,
{
    fn identity(&self) -> IdentityKeyRef;
}

impl IntoGatewayNode for GatewayBond {
    fn identity(&self) -> IdentityKeyRef {
        &self.gateway.identity_key
    }
}

impl IntoGatewayNode for DescribedGateway {
    fn identity(&self) -> IdentityKeyRef {
        &self.bond.gateway.identity_key
    }
}

pub fn nym_topology_from_detailed<G>(
    mix_details: Vec<MixNodeDetails>,
    gateway_bonds: Vec<G>,
) -> NymTopology
where
    G: IntoGatewayNode,
    <G as TryInto<gateway::Node>>::Error: Display,
{
    let mut mixes = BTreeMap::new();
    for bond in mix_details
        .into_iter()
        .map(|details| details.bond_information)
    {
        let layer = bond.layer as MixLayer;
        if layer == 0 || layer > 3 {
            warn!(
                "{} says it's on invalid layer {layer}!",
                bond.mix_node.identity_key
            );
            continue;
        }
        let mix_id = bond.mix_id;
        let mix_identity = bond.mix_node.identity_key.clone();

        let layer_entry = mixes.entry(layer).or_insert_with(Vec::new);
        match bond.try_into() {
            Ok(mix) => layer_entry.push(mix),
            Err(err) => {
                warn!("Mix {mix_id} / {mix_identity} is malformed: {err}");
                continue;
            }
        }
    }

    let mut gateways = Vec::with_capacity(gateway_bonds.len());
    for bond in gateway_bonds.into_iter() {
        let gate_id = bond.identity().to_owned();
        match bond.try_into() {
            Ok(gate) => gateways.push(gate),
            Err(err) => {
                warn!("Gateway {gate_id} is malformed: {err}");
                continue;
            }
        }
    }

    NymTopology::new(mixes, gateways)
}

#[cfg(test)]
mod converting_mixes_to_vec {
    use super::*;

    #[cfg(test)]
    mod when_nodes_exist {
        use nym_crypto::asymmetric::{encryption, identity};

        use super::*;
        use nym_mixnet_contract_common::Layer;

        #[test]
        fn returns_a_vec_with_hashmap_values() {
            let node1 = mix::Node {
                mix_id: 42,
                owner: "N/A".to_string(),
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
                layer: Layer::One,
                version: "0.2.0".into(),
            };

            let node2 = mix::Node {
                owner: "Alice".to_string(),
                ..node1.clone()
            };

            let node3 = mix::Node {
                owner: "Bob".to_string(),
                ..node1.clone()
            };

            let mut mixes: BTreeMap<MixLayer, Vec<mix::Node>> = BTreeMap::new();
            mixes.insert(1, vec![node1, node2]);
            mixes.insert(2, vec![node3]);

            let topology = NymTopology::new(mixes, vec![]);
            let mixvec = topology.mixes_as_vec();
            assert!(mixvec.iter().any(|node| node.owner == "N/A"));
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
