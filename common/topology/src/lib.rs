// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::filter::VersionFilterable;
use log::warn;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::GatewayBond;
use nymsphinx_addressing::nodes::NodeIdentity;
use nymsphinx_types::Node as SphinxNode;
use rand::Rng;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;

pub mod filter;
pub mod gateway;
pub mod mix;

#[derive(Debug)]
pub enum NymTopologyError {
    InvalidMixLayerError,
    MissingLayerError(Vec<u64>),
    NonExistentGatewayError,

    InvalidNumberOfHopsError,
    NoMixesOnLayerAvailable(MixLayer),
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
    mixes: HashMap<MixLayer, Vec<mix::Node>>,
    gateways: Vec<gateway::Node>,
}

impl NymTopology {
    pub fn new(mixes: HashMap<MixLayer, Vec<mix::Node>>, gateways: Vec<gateway::Node>) -> Self {
        NymTopology { mixes, gateways }
    }

    pub fn mixes(&self) -> &HashMap<MixLayer, Vec<mix::Node>> {
        &self.mixes
    }

    pub fn mixes_as_vec(&self) -> Vec<mix::Node> {
        let mut mixes: Vec<mix::Node> = vec![];

        for layer in self.mixes().values() {
            mixes.extend(layer.to_owned())
        }
        mixes
    }

    pub fn mixes_in_layer(&self, layer: MixLayer) -> Vec<mix::Node> {
        assert!(vec![1, 2, 3].contains(&layer));
        self.mixes.get(&layer).unwrap().to_owned()
    }

    pub fn gateways(&self) -> &[gateway::Node] {
        &self.gateways
    }

    fn get_gateway(&self, gateway_identity: &NodeIdentity) -> Option<&gateway::Node> {
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

    /// Returns a vec of size of `num_mix_hops` of mixnodes, such that each subsequent node is on
    /// next layer, starting from layer 1
    pub fn random_mix_route<R>(
        &self,
        rng: &mut R,
        num_mix_hops: u8,
    ) -> Result<Vec<SphinxNode>, NymTopologyError>
    where
        // I don't think there's a need for this RNG to be crypto-secure
        R: Rng + ?Sized,
    {
        use rand::seq::SliceRandom;

        if self.mixes.len() < num_mix_hops as usize {
            return Err(NymTopologyError::InvalidNumberOfHopsError);
        }
        let mut route = Vec::with_capacity(num_mix_hops as usize);

        // there is no "layer 0"
        for layer in 1..=num_mix_hops {
            // get all mixes on particular layer
            let layer_mixes = self
                .mixes
                .get(&layer)
                .ok_or(NymTopologyError::NoMixesOnLayerAvailable(layer))?;

            // choose a random mix from the above list
            // this can return a 'None' only if slice is empty
            let random_mix = layer_mixes
                .choose(rng)
                .ok_or(NymTopologyError::NoMixesOnLayerAvailable(layer))?;
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
        // I don't think there's a need for this RNG to be crypto-secure
        R: Rng + ?Sized,
    {
        let gateway = self
            .get_gateway(gateway_identity)
            .ok_or(NymTopologyError::NonExistentGatewayError)?;

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
    pub fn can_construct_path_through(&self, num_mix_hops: u8) -> bool {
        // if there are no gateways present, we can't do anything
        if self.gateways.is_empty() {
            return false;
        }

        // early termination
        if self.mixes.is_empty() {
            return false;
        }

        // make sure there's at least one mix per layer
        for i in 1..=num_mix_hops {
            match self.mixes.get(&i) {
                None => return false,
                Some(layer_entry) => {
                    if layer_entry.is_empty() {
                        return false;
                    }
                }
            }
        }
        true
    }

    #[must_use]
    pub fn filter_system_version(&self, expected_version: &str) -> Self {
        self.filter_node_versions(expected_version, expected_version)
    }

    #[must_use]
    pub fn filter_node_versions(
        &self,
        expected_mix_version: &str,
        expected_gateway_version: &str,
    ) -> Self {
        NymTopology {
            mixes: self.mixes.filter_by_version(expected_mix_version),
            gateways: self.gateways.filter_by_version(expected_gateway_version),
        }
    }
}

pub fn nym_topology_from_detailed(
    mix_details: Vec<MixNodeDetails>,
    gateway_bonds: Vec<GatewayBond>,
) -> NymTopology {
    let mut mixes = HashMap::new();
    for bond in mix_details
        .into_iter()
        .map(|details| details.bond_information)
    {
        let layer = bond.layer as MixLayer;
        if layer == 0 || layer > 3 {
            warn!(
                "{} says it's on invalid layer {}!",
                bond.mix_node.identity_key, layer
            );
            continue;
        }
        let mix_id = bond.id;
        let mix_identity = bond.mix_node.identity_key.clone();

        let layer_entry = mixes.entry(layer).or_insert_with(Vec::new);
        match bond.try_into() {
            Ok(mix) => layer_entry.push(mix),
            Err(err) => {
                warn!("Mix {} / {} is malformed - {}", mix_id, mix_identity, err);
                continue;
            }
        }
    }

    let mut gateways = Vec::with_capacity(gateway_bonds.len());
    for bond in gateway_bonds.into_iter() {
        let gate_id = bond.gateway.identity_key.clone();
        match bond.try_into() {
            Ok(gate) => gateways.push(gate),
            Err(err) => {
                warn!("Gateway {} is malformed - {}", gate_id, err);
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
        use crypto::asymmetric::{encryption, identity};

        use super::*;
        use mixnet_contract_common::Layer;

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
                version: "0.x.0".to_string(),
            };

            let node2 = mix::Node {
                owner: "Alice".to_string(),
                ..node1.clone()
            };

            let node3 = mix::Node {
                owner: "Bob".to_string(),
                ..node1.clone()
            };

            let mut mixes: HashMap<MixLayer, Vec<mix::Node>> = HashMap::new();
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
            let topology = NymTopology::new(HashMap::new(), vec![]);
            let mixvec = topology.mixes_as_vec();
            assert!(mixvec.is_empty());
        }
    }
}
