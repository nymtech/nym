use crate::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::{Client, Config, DirectoryClient};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryInto;
use std::io;
use std::net::ToSocketAddrs;
use topology::coco;
use topology::mix;
use topology::provider;
use topology::NymTopology;

// special version of 'PartialEq' that does not care about last_seen field
//(where applicable) or order of elements in vectors
trait PresenceEq {
    fn presence_eq(&self, other: &Self) -> bool;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::coco::Node> for CocoPresence {
    fn into(self) -> topology::coco::Node {
        topology::coco::Node {
            host: self.host,
            pub_key: self.pub_key,
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

impl From<topology::coco::Node> for CocoPresence {
    fn from(cn: coco::Node) -> Self {
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

impl TryInto<topology::mix::Node> for MixNodePresence {
    type Error = io::Error;

    fn try_into(self) -> Result<topology::mix::Node, Self::Error> {
        let resolved_hostname = self.host.to_socket_addrs()?.next();
        if resolved_hostname.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "no valid socket address",
            ));
        }

        Ok(topology::mix::Node {
            host: resolved_hostname.unwrap(),
            pub_key: self.pub_key,
            layer: self.layer,
            last_seen: self.last_seen,
            version: self.version,
        })
    }
}

impl From<topology::mix::Node> for MixNodePresence {
    fn from(mn: mix::Node) -> Self {
        MixNodePresence {
            host: mn.host.to_string(),
            pub_key: mn.pub_key,
            layer: mn.layer,
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

impl PresenceEq for MixProviderPresence {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.registered_clients.len() != other.registered_clients.len() {
            return false;
        }

        if self.client_listener != other.client_listener
            || self.mixnet_listener != other.mixnet_listener
            || self.pub_key != other.pub_key
            || self.version != other.version
        {
            return false;
        }

        let clients_self_set: HashSet<_> =
            self.registered_clients.iter().map(|c| &c.pub_key).collect();
        let clients_other_set: HashSet<_> = other
            .registered_clients
            .iter()
            .map(|c| &c.pub_key)
            .collect();

        clients_self_set == clients_other_set
    }
}

impl Into<topology::provider::Node> for MixProviderPresence {
    fn into(self) -> topology::provider::Node {
        topology::provider::Node {
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

impl From<topology::provider::Node> for MixProviderPresence {
    fn from(mpn: provider::Node) -> Self {
        MixProviderPresence {
            client_listener: mpn.client_listener.to_string(),
            mixnet_listener: mpn.mixnet_listener.to_string(),
            pub_key: mpn.pub_key,
            registered_clients: mpn
                .registered_clients
                .into_iter()
                .map(|c| c.into())
                .collect(),
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

impl Into<topology::provider::Client> for MixProviderClient {
    fn into(self) -> topology::provider::Client {
        topology::provider::Client {
            pub_key: self.pub_key,
        }
    }
}

impl From<topology::provider::Client> for MixProviderClient {
    fn from(mpc: topology::provider::Client) -> Self {
        MixProviderClient {
            pub_key: mpc.pub_key,
        }
    }
}

impl PresenceEq for Vec<CocoPresence> {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // we can't take the whole thing into set as it does not implement 'Eq' and we can't
        // derive it as we don't want to take 'last_seen' into consideration
        let self_set: HashSet<_> = self
            .iter()
            .map(|c| (&c.host, &c.pub_key, &c.version))
            .collect();
        let other_set: HashSet<_> = other
            .iter()
            .map(|c| (&c.host, &c.pub_key, &c.version))
            .collect();

        self_set == other_set
    }
}

impl PresenceEq for Vec<MixNodePresence> {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // we can't take the whole thing into set as it does not implement 'Eq' and we can't
        // derive it as we don't want to take 'last_seen' into consideration
        let self_set: HashSet<_> = self
            .iter()
            .map(|m| (&m.host, &m.pub_key, &m.version, &m.layer))
            .collect();
        let other_set: HashSet<_> = other
            .iter()
            .map(|m| (&m.host, &m.pub_key, &m.version, &m.layer))
            .collect();

        self_set == other_set
    }
}

impl PresenceEq for Vec<MixProviderPresence> {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // we can't take the whole thing into set as it does not implement 'Eq' and we can't
        // derive it as we don't want to take 'last_seen' into consideration.
        // We also don't care about order of registered_clients

        // since we're going to be getting rid of this very soon anyway, just clone registered
        // clients vector and sort it

        let self_set: HashSet<_> = self
            .iter()
            .map(|p| {
                (
                    &p.client_listener,
                    &p.mixnet_listener,
                    &p.pub_key,
                    &p.version,
                    p.registered_clients
                        .iter()
                        .cloned()
                        .map(|c| c.pub_key)
                        .collect::<Vec<_>>()
                        .sort(),
                )
            })
            .collect();
        let other_set: HashSet<_> = other
            .iter()
            .map(|p| {
                (
                    &p.client_listener,
                    &p.mixnet_listener,
                    &p.pub_key,
                    &p.version,
                    p.registered_clients
                        .iter()
                        .cloned()
                        .map(|c| c.pub_key)
                        .collect::<Vec<_>>()
                        .sort(),
                )
            })
            .collect();

        self_set == other_set
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

impl PartialEq for Topology {
    // we need a custom implementation as the order of nodes does not matter
    // also we do not care about 'last_seen' when comparing topologies
    fn eq(&self, other: &Self) -> bool {
        if !self.coco_nodes.presence_eq(&other.coco_nodes)
            || !self.mix_nodes.presence_eq(&other.mix_nodes)
            || !self
                .mix_provider_nodes
                .presence_eq(&other.mix_provider_nodes)
        {
            return false;
        }

        true
    }
}

impl NymTopology for Topology {
    fn new(directory_server: String) -> Self {
        debug!("Using directory server: {:?}", directory_server);
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

        let mix_presence = MixNodePresence {
            host: unresolvable_hostname.to_string(),
            pub_key: "".to_string(),
            layer: 0,
            last_seen: 0,
            version: "".to_string(),
        };

        let result: Result<mix::Node, io::Error> = mix_presence.try_into();
        assert!(result.is_err())
    }

    #[test]
    fn it_returns_resolved_ip_on_resolvable_hostname() {
        let resolvable_hostname = "nymtech.net:1234";

        let mix_presence = MixNodePresence {
            host: resolvable_hostname.to_string(),
            pub_key: "".to_string(),
            layer: 0,
            last_seen: 0,
            version: "".to_string(),
        };

        let result: Result<topology::mix::Node, io::Error> = mix_presence.try_into();
        assert!(result.is_ok())
    }
}
