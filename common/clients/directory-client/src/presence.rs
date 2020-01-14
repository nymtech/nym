use crate::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::{Client, Config, DirectoryClient};
use serde::{Deserialize, Serialize};
use topology::{NymTopology, MixProviderNode, MixNode, CocoNode};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::CocoNode> for CocoPresence {
    fn into(self) -> topology::CocoNode {
        topology::CocoNode {
            host: self.host,
            pub_key: self.pub_key,
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

impl From<topology::CocoNode> for CocoPresence {
    fn from(cn: CocoNode) -> Self {
        CocoPresence {
            host: cn.host,
            pub_key: cn.pub_key,
            last_seen: cn.last_seen,
            version: cn.version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixNodePresence {
    pub host: String,
    pub pub_key: String,
    pub layer: u64,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::MixNode> for MixNodePresence {
    fn into(self) -> topology::MixNode {
        topology::MixNode {
            host: self.host.parse().unwrap(),
            pub_key: self.pub_key,
            layer: self.layer,
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

impl From<topology::MixNode> for MixNodePresence {
    fn from(mn: MixNode) -> Self {
        MixNodePresence {
            host: mn.host.to_string(),
            pub_key: mn.pub_key,
            layer: mn.last_seen,
            last_seen: mn.last_seen,
            version: mn.version,
        }
    }
}


#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderPresence {
    pub client_listener: String,
    pub mixnet_listener: String,
    pub pub_key: String,
    pub registered_clients: Vec<MixProviderClient>,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::MixProviderNode> for MixProviderPresence {
    fn into(self) -> topology::MixProviderNode {
        topology::MixProviderNode {
            client_listener: self.client_listener.parse().unwrap(),
            mixnet_listener: self.mixnet_listener.parse().unwrap(),
            pub_key: self.pub_key,
            registered_clients: self
                .registered_clients
                .into_iter()
                .map(|c| c.into())
                .collect(),
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

impl From<topology::MixProviderNode> for MixProviderPresence {
    fn from(mpn: MixProviderNode) -> Self {
        MixProviderPresence {
            client_listener: mpn.client_listener.to_string(),
            mixnet_listener: mpn.mixnet_listener.to_string(),
            pub_key: mpn.pub_key,
            registered_clients: mpn.registered_clients.into_iter().map(|c| c.into()).collect(),
            last_seen: mpn.last_seen,
            version: mpn.version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderClient {
    pub pub_key: String,
}

impl Into<topology::MixProviderClient> for MixProviderClient {
    fn into(self) -> topology::MixProviderClient {
        topology::MixProviderClient {
            pub_key: self.pub_key,
        }
    }
}

impl From<topology::MixProviderClient> for MixProviderClient {
    fn from(mpc: topology::MixProviderClient) -> Self {
        MixProviderClient {
            pub_key: mpc.pub_key,
        }
    }
}

// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub coco_nodes: Vec<CocoPresence>,
    pub mix_nodes: Vec<MixNodePresence>,
    pub mix_provider_nodes: Vec<MixProviderPresence>,
}

impl NymTopology for Topology {
    fn new(directory_server: String) -> Self {
        println!("Using directory server: {:?}", directory_server);
        let directory_config = Config {
            base_url: directory_server,
        };
        let directory = Client::new(directory_config);

        let topology = directory
            .presence_topology
            .get()
            .expect("Failed to retrieve network topology.");
        topology
    }

    fn new_from_nodes(mix_nodes: Vec<MixNode>, mix_provider_nodes: Vec<MixProviderNode>, coco_nodes: Vec<CocoNode>) -> Self {
        Topology {
            coco_nodes: coco_nodes.into_iter().map(|node| node.into()).collect(),
            mix_nodes: mix_nodes.into_iter().map(|node| node.into()).collect(),
            mix_provider_nodes: mix_provider_nodes.into_iter().map(|node| node.into()).collect(),
        }
    }

    fn get_mix_nodes(&self) -> Vec<topology::MixNode> {
        self.mix_nodes.iter().map(|x| x.clone().into()).collect()
    }

    fn get_mix_provider_nodes(&self) -> Vec<topology::MixProviderNode> {
        self.mix_provider_nodes
            .iter()
            .map(|x| x.clone().into())
            .collect()
    }

    fn get_coco_nodes(&self) -> Vec<topology::CocoNode> {
        self.coco_nodes.iter().map(|x| x.clone().into()).collect()
    }
}
