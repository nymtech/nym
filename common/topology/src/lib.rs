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
use nymsphinx_addressing::nodes::NodeIdentity;
use nymsphinx_types::Node as SphinxNode;
use rand::Rng;
use std::collections::HashMap;

pub mod coco;
mod filter;
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

#[derive(Debug)]
pub struct NymTopology {
    coco_nodes: Vec<coco::Node>,
    mixes: HashMap<MixLayer, Vec<mix::Node>>,
    gateways: Vec<gateway::Node>,
}

impl NymTopology {
    pub fn new(
        coco_nodes: Vec<coco::Node>,
        mixes: HashMap<MixLayer, Vec<mix::Node>>,
        gateways: Vec<gateway::Node>,
    ) -> Self {
        NymTopology {
            coco_nodes,
            mixes,
            gateways,
        }
    }

    pub fn coco_nodes(&self) -> &Vec<coco::Node> {
        &self.coco_nodes
    }

    pub fn mixes(&self) -> &HashMap<MixLayer, Vec<mix::Node>> {
        &self.mixes
    }

    pub fn gateways(&self) -> &Vec<gateway::Node> {
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
            let layer_mixes = match self.mixes.get(&layer) {
                Some(mixes) => mixes,
                None => return Err(NymTopologyError::NoMixesOnLayerAvailable(layer)),
            };

            // choose a random mix from the above list
            // this can return a 'None' only if slice is empty
            let random_mix = match layer_mixes.choose(rng) {
                Some(random_mix) => random_mix,
                None => return Err(NymTopologyError::NoMixesOnLayerAvailable(layer)),
            };
            route.push(random_mix.into());
        }

        Ok(route)
    }

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
            .ok_or_else(|| NymTopologyError::NonExistentGatewayError)?;

        Ok(self
            .random_mix_route(rng, num_mix_hops)?
            .into_iter()
            .chain(std::iter::once(gateway.into()))
            .collect())
    }

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
        self.filter_node_versions(expected_version, expected_version, expected_version)
    }

    pub fn filter_node_versions(
        &self,
        expected_mix_version: &str,
        expected_gateway_version: &str,
        expected_coco_version: &str,
    ) -> Self {
        NymTopology {
            mixes: self.mixes.filter_by_version(expected_mix_version),
            gateways: self.gateways.filter_by_version(expected_gateway_version),
            coco_nodes: self.coco_nodes.filter_by_version(expected_coco_version),
        }
    }
}
