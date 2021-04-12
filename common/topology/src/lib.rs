// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::filter::VersionFilterable;
use log::warn;
use mixnet_contract::{GatewayBond, MixNodeBond};
use nymsphinx_addressing::nodes::NodeIdentity;
use nymsphinx_types::Node as SphinxNode;
use rand::Rng;
use std::collections::HashMap;
use std::convert::TryInto;

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

    pub fn filter_system_version(&self, expected_version: &str) -> Self {
        self.filter_node_versions(expected_version, expected_version)
    }

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

pub fn nym_topology_from_bonds(
    mix_bonds: Vec<MixNodeBond>,
    gateway_bonds: Vec<GatewayBond>,
) -> NymTopology {
    let mut mixes = HashMap::new();
    for bond in mix_bonds.into_iter() {
        let layer = bond.mix_node.layer as MixLayer;
        if layer == 0 || layer > 3 {
            warn!(
                "{} says it's on invalid layer {}!",
                bond.mix_node.identity_key, layer
            );
            continue;
        }
        let mix_id = bond.mix_node.identity_key.clone();

        let layer_entry = mixes.entry(layer).or_insert_with(Vec::new);
        match bond.try_into() {
            Ok(mix) => layer_entry.push(mix),
            Err(err) => {
                warn!("Mix {} is malformed - {}", mix_id, err);
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

        #[test]
        fn returns_a_vec_with_hashmap_values() {
            let node1 = mix::Node {
                owner: "N/A".to_string(),
                stake: 0,
                location: "London".to_string(),
                host: "3.3.3.3:1789".parse().unwrap(),
                identity_key: identity::PublicKey::from_base58_string(
                    "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "C7cown6dYCLZpLiMFC1PaBmhvLvmJmLDJGeRTbPD45bX",
                )
                .unwrap(),
                layer: 1,
                version: "0.x.0".to_string(),
            };

            let node2 = mix::Node {
                location: "Thunder Bay".to_string(),
                ..node1.clone()
            };

            let node3 = mix::Node {
                location: "Warsaw".to_string(),
                ..node1.clone()
            };

            let mut mixes: HashMap<MixLayer, Vec<mix::Node>> = HashMap::new();
            mixes.insert(1, vec![node1, node2]);
            mixes.insert(2, vec![node3]);

            let topology = NymTopology::new(mixes, vec![]);
            let mixvec = topology.mixes_as_vec();
            assert!(mixvec.iter().any(|node| node.location == "London"));
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
