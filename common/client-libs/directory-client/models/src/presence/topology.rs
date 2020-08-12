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

use super::{coconodes, gateways, mixnodes, providers};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use topology::{MixLayer, NymTopology};

#[derive(Debug)]
pub enum TopologyConversionError {
    CocoError(self::coconodes::ConversionError),
    GatewayError(self::gateways::ConversionError),
    MixError(self::mixnodes::ConversionError),
}

impl From<self::coconodes::ConversionError> for TopologyConversionError {
    fn from(err: self::coconodes::ConversionError) -> Self {
        TopologyConversionError::CocoError(err)
    }
}

impl From<self::gateways::ConversionError> for TopologyConversionError {
    fn from(err: self::gateways::ConversionError) -> Self {
        TopologyConversionError::GatewayError(err)
    }
}

impl From<self::mixnodes::ConversionError> for TopologyConversionError {
    fn from(err: self::mixnodes::ConversionError) -> Self {
        TopologyConversionError::MixError(err)
    }
}

// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub coco_nodes: Vec<coconodes::CocoPresence>,
    pub mix_nodes: Vec<mixnodes::MixNodePresence>,
    pub mix_provider_nodes: Vec<providers::MixProviderPresence>,
    pub gateway_nodes: Vec<gateways::GatewayPresence>,
}

impl TryInto<NymTopology> for Topology {
    type Error = TopologyConversionError;

    fn try_into(self) -> Result<NymTopology, TopologyConversionError> {
        use std::collections::HashMap;

        let mut coco_nodes = Vec::with_capacity(self.coco_nodes.len());
        for coco in self.coco_nodes.into_iter() {
            coco_nodes.push(coco.try_into()?)
        }

        let mut mixes = HashMap::new();
        for mix in self.mix_nodes.into_iter() {
            let layer = mix.layer as MixLayer;
            let layer_entry = mixes.entry(layer).or_insert(Vec::new());
            layer_entry.push(mix.try_into()?)
        }

        let mut gateways = Vec::with_capacity(self.gateway_nodes.len());
        for gate in self.gateway_nodes.into_iter() {
            gateways.push(gate.try_into()?)
        }

        Ok(NymTopology::new(coco_nodes, mixes, gateways))
    }
}

#[cfg(test)]
mod converting_mixnode_presence_into_topology_mixnode {
    use super::*;

    #[test]
    fn it_returns_error_on_unresolvable_hostname() {
        use topology::mix;

        let unresolvable_hostname = "foomp.foomp.foomp:1234";

        let mix_presence = mixnodes::MixNodePresence {
            location: "".to_string(),
            host: unresolvable_hostname.to_string(),
            pub_key: "BnLYqQjb8K6TmW5oFdNZrUTocGxa3rgzBvapQrf8XUbF".to_string(),
            layer: 0,
            last_seen: 0,
            version: "".to_string(),
        };

        let result: Result<mix::Node, self::mixnodes::ConversionError> = mix_presence.try_into();
        assert!(result.is_err());
    }

    #[test]
    #[cfg_attr(feature = "offline-test", ignore)]
    fn it_returns_resolved_ip_on_resolvable_hostname() {
        let resolvable_hostname = "nymtech.net:1234";

        let mix_presence = mixnodes::MixNodePresence {
            location: "".to_string(),
            host: resolvable_hostname.to_string(),
            pub_key: "BnLYqQjb8K6TmW5oFdNZrUTocGxa3rgzBvapQrf8XUbF".to_string(),
            layer: 0,
            last_seen: 0,
            version: "".to_string(),
        };

        let result: Result<topology::mix::Node, self::mixnodes::ConversionError> =
            mix_presence.try_into();
        result.unwrap();
        // assert!(result.is_ok())
    }
}
