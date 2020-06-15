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
use crate::{Client, Config, DirectoryClient};
use futures::future::BoxFuture;
use log::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use topology::{coco, gateway, mix, provider, NymTopology};
// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub coco_nodes: Vec<coconodes::CocoPresence>,
    pub mix_nodes: Vec<mixnodes::MixNodePresence>,
    pub mix_provider_nodes: Vec<providers::MixProviderPresence>,
    pub gateway_nodes: Vec<gateways::GatewayPresence>,
}

impl NymTopology for Topology {
    // TODO: this will need some changes to not imply having to make an HTTP request in constructor
    // TODO2: and also be reworked/removed with the topology re-work
    fn new<'a>(directory_server: String) -> BoxFuture<'a, Self> {
        debug!("Using directory server: {:?}", directory_server);
        let directory_config = Config {
            base_url: directory_server,
        };
        let directory = Client::new(directory_config);
        Box::pin({
            async move {
                directory
                    .get_topology()
                    .await
                    .expect("Failed to retrieve network topology.")
            }
        })
    }

    fn new_from_nodes(
        mix_nodes: Vec<mix::Node>,
        mix_provider_nodes: Vec<provider::Node>,
        coco_nodes: Vec<coco::Node>,
        gateway_nodes: Vec<gateway::Node>,
    ) -> Self {
        Topology {
            coco_nodes: coco_nodes.into_iter().map(|node| node.into()).collect(),
            mix_nodes: mix_nodes.into_iter().map(|node| node.into()).collect(),
            mix_provider_nodes: mix_provider_nodes
                .into_iter()
                .map(|node| node.into())
                .collect(),
            gateway_nodes: gateway_nodes.into_iter().map(|node| node.into()).collect(),
        }
    }

    fn mix_nodes(&self) -> Vec<mix::Node> {
        self.mix_nodes
            .iter()
            .filter_map(|x| x.clone().try_into().ok())
            .collect()
    }

    fn providers(&self) -> Vec<provider::Node> {
        self.mix_provider_nodes
            .iter()
            .map(|x| x.clone().into())
            .collect()
    }

    fn gateways(&self) -> Vec<gateway::Node> {
        self.gateway_nodes
            .iter()
            .map(|x| x.clone().into())
            .collect()
    }

    fn coco_nodes(&self) -> Vec<topology::coco::Node> {
        self.coco_nodes.iter().map(|x| x.clone().into()).collect()
    }
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

        let result: Result<mix::Node, std::io::Error> = mix_presence.try_into();
        assert!(result.is_err()) // This fails only for me. Why?
                                 // ¯\_(ツ)_/¯ - works on my machine (and travis)
                                 // Is it still broken?
    }

    #[test]
    #[cfg_attr(feature = "offline-test", ignore)]
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
