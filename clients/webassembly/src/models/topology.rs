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

use nymsphinx::NodeAddressBytes;
use serde::Serializer;
use topology::{coco, gateway, mix, provider, NymTopology};

#[derive(Clone, Debug)]
pub struct Topology {
    inner: directory_client_models::presence::Topology,
}

impl serde::Serialize for Topology {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl Topology {
    pub fn new(json: &str) -> Self {
        if json.is_empty() {
            panic!("empty json passed");
        }

        Topology {
            inner: serde_json::from_str(json).unwrap(),
        }
    }

    pub(crate) fn random_route_to_gateway_by_address(
        &self,
        gateway: &NodeAddressBytes,
    ) -> Option<Vec<nymsphinx::Node>> {
        let b58_address = gateway.to_base58_string();

        let gateway = self
            .gateways()
            .iter()
            .cloned()
            .find(|gateway| gateway.pub_key == b58_address.clone())?;

        self.random_route_to_gateway(gateway.into()).ok()
    }

    #[cfg(test)]
    pub(crate) fn set_mixnodes(
        &mut self,
        mix_nodes: Vec<directory_client_models::presence::mixnodes::MixNodePresence>,
    ) {
        self.inner.mix_nodes = mix_nodes
    }

    #[cfg(test)]
    pub(crate) fn get_current_raw_mixnodes(
        &self,
    ) -> Vec<directory_client_models::presence::mixnodes::MixNodePresence> {
        self.inner.mix_nodes.clone()
    }
}

impl NymTopology for Topology {
    fn new_from_nodes(
        mix_nodes: Vec<mix::Node>,
        mix_provider_nodes: Vec<provider::Node>,
        coco_nodes: Vec<coco::Node>,
        gateway_nodes: Vec<gateway::Node>,
    ) -> Self {
        Topology {
            inner: directory_client_models::presence::Topology::new_from_nodes(
                mix_nodes,
                mix_provider_nodes,
                coco_nodes,
                gateway_nodes,
            ),
        }
    }
    fn mix_nodes(&self) -> Vec<mix::Node> {
        self.inner.mix_nodes()
    }

    fn providers(&self) -> Vec<provider::Node> {
        self.inner.providers()
    }

    fn gateways(&self) -> Vec<gateway::Node> {
        self.inner.gateways()
    }

    fn coco_nodes(&self) -> Vec<topology::coco::Node> {
        self.inner.coco_nodes()
    }
}
