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

use nymsphinx::DestinationAddressBytes;
use nymsphinx::Node as SphinxNode;
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::HashMap;
use std::convert::TryInto;
use topology::{gateway, mix, provider};

pub mod coconodes;
pub mod gateways;
pub mod keys;
pub mod mixnodes;
pub mod providers;

// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub coco_nodes: Vec<coconodes::CocoPresence>,
    pub mix_nodes: Vec<mixnodes::MixNodePresence>,
    pub mix_provider_nodes: Vec<providers::MixProviderPresence>,
    pub gateway_nodes: Vec<gateways::GatewayPresence>,
}

impl Topology {
    pub fn new(json: &str) -> Self {
        serde_json::from_str(json).unwrap()
    }

    fn make_layered_topology(&self) -> Result<HashMap<u64, Vec<mix::Node>>, NymTopologyError> {
        let mut layered_topology: HashMap<u64, Vec<mix::Node>> = HashMap::new();
        let mut highest_layer = 0;
        for mix in self.mix_nodes() {
            // we need to have extra space for provider
            if mix.layer > nymsphinx::MAX_PATH_LENGTH as u64 {
                return Err(NymTopologyError::InvalidMixLayerError);
            }
            highest_layer = max(highest_layer, mix.layer);

            let layer_nodes = layered_topology.entry(mix.layer).or_insert_with(Vec::new);
            layer_nodes.push(mix);
        }

        // verify the topology - make sure there are no gaps and there is at least one node per layer
        let mut missing_layers = Vec::new();
        for layer in 1..=highest_layer {
            if !layered_topology.contains_key(&layer) {
                missing_layers.push(layer);
            }
            if layered_topology[&layer].is_empty() {
                missing_layers.push(layer);
            }
        }

        if !missing_layers.is_empty() {
            return Err(NymTopologyError::MissingLayerError(missing_layers));
        }

        Ok(layered_topology)
    }

    // Tries to get a route through the mix network
    fn random_mix_route(&self) -> Result<Vec<SphinxNode>, NymTopologyError> {
        let mut layered_topology = self.make_layered_topology()?;
        let num_layers = layered_topology.len();
        let route = (1..=num_layers as u64)
            // unwrap is safe for 'remove' as it it failed, it implied the entry never existed
            // in the map in the first place which would contradict what we've just done
            .map(|layer| layered_topology.remove(&layer).unwrap()) // for each layer
            .map(|nodes| nodes.into_iter().choose(&mut rand::thread_rng()).unwrap()) // choose random node
            .map(|random_node| random_node.into()) // and convert it into sphinx specific node format
            .collect();

        Ok(route)
    }

    // Sets up a route to a specific provider
    pub fn random_route_to(
        &self,
        gateway_node: SphinxNode,
    ) -> Result<Vec<SphinxNode>, NymTopologyError> {
        Ok(self
            .random_mix_route()?
            .into_iter()
            .chain(std::iter::once(gateway_node))
            .collect())
    }

    pub fn mix_nodes(&self) -> Vec<mix::Node> {
        self.mix_nodes
            .iter()
            .filter_map(|x| x.clone().try_into().ok())
            .collect()
    }

    pub fn providers(&self) -> Vec<provider::Node> {
        self.mix_provider_nodes
            .iter()
            .map(|x| x.clone().into())
            .collect()
    }

    pub fn gateways(&self) -> Vec<gateway::Node> {
        self.gateway_nodes
            .iter()
            .map(|x| x.clone().into())
            .collect()
    }

    pub(crate) fn random_route_to_client(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Option<Vec<SphinxNode>> {
        let b58_address = client_address.to_base58_string();

        let gateway = self
            .gateways()
            .iter()
            .cloned()
            .find(|gateway| gateway.has_client(b58_address.clone()))?;

        self.random_route_to(gateway.into()).ok()
    }
}

#[derive(Debug)]
pub enum NymTopologyError {
    InvalidMixLayerError,
    MissingLayerError(Vec<u64>),
}

#[cfg(test)]
mod converting_mixnode_presence_into_topology_mixnode {
    use super::*;

    #[test]
    fn it_returns_error_on_unresolvable_hostname() {
        let unresolvable_hostname = "foomp.foomp.foomp:1234";

        let mix_presence = mixnodes::MixNodePresence {
            location: "".to_string(),
            host: unresolvable_hostname.to_string(),
            pub_key: "".to_string(),
            layer: 0,
            last_seen: 0,
            version: "".to_string(),
        };

        let _result: Result<mix::Node, std::io::Error> = mix_presence.try_into();
        // assert!(result.is_err()) // This fails only for me. Why?
        // ¯\_(ツ)_/¯ - works on my machine (and travis)
    }

    #[test]
    fn it_returns_resolved_ip_on_resolvable_hostname() {
        let resolvable_hostname = "nymtech.net:1234";

        let mix_presence = mixnodes::MixNodePresence {
            location: "".to_string(),
            host: resolvable_hostname.to_string(),
            pub_key: "".to_string(),
            layer: 0,
            last_seen: 0,
            version: "".to_string(),
        };

        let result: Result<topology::mix::Node, std::io::Error> = mix_presence.try_into();
        assert!(result.is_ok())
    }
}
