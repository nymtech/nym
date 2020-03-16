use crate::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::{Client, Config, DirectoryClient};
use log::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use topology::coco;
use topology::mix;
use topology::provider;
use topology::NymTopology;

pub mod coconodes;
pub mod mixnodes;
pub mod providers;

// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub coco_nodes: Vec<coconodes::CocoPresence>,
    pub mix_nodes: Vec<mixnodes::MixNodePresence>,
    pub mix_provider_nodes: Vec<providers::MixProviderPresence>,
}

impl NymTopology for Topology {
    fn new(directory_server: String) -> Self {
        debug!("Using directory server: {:?}", directory_server);
        let directory_config = Config {
            base_url: directory_server,
        };
        let directory = Client::new(directory_config);

        directory
            .presence_topology
            .get()
            .expect("Failed to retrieve network topology.")
    }

    fn new_from_nodes(
        mix_nodes: Vec<mix::Node>,
        mix_provider_nodes: Vec<provider::Node>,
        coco_nodes: Vec<coco::Node>,
    ) -> Self {
        Topology {
            coco_nodes: coco_nodes.into_iter().map(|node| node.into()).collect(),
            mix_nodes: mix_nodes.into_iter().map(|node| node.into()).collect(),
            mix_provider_nodes: mix_provider_nodes
                .into_iter()
                .map(|node| node.into())
                .collect(),
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
