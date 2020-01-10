use addressing;
use curve25519_dalek::montgomery::MontgomeryPoint;
use rand::seq::SliceRandom;
use sphinx::route::{Node as SphinxNode, NodeAddressBytes};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct MixNode {
    pub host: SocketAddr,
    pub pub_key: String,
    pub layer: u64,
    pub last_seen: u64,
    pub version: String,
}

#[derive(Debug)]
pub struct MixProviderClient {
    pub pub_key: String,
}

#[derive(Debug)]
pub struct MixProviderNode {
    pub client_listener: SocketAddr,
    pub mixnet_listener: SocketAddr,
    pub pub_key: String,
    pub registered_clients: Vec<MixProviderClient>,
    pub last_seen: u64,
    pub version: String,
}

#[derive(Debug)]
pub struct CocoNode {
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
}

pub trait NymTopology {
    fn new(directory_server: String) -> Self;
    fn get_mix_nodes(&self) -> Vec<MixNode>;
    fn get_mix_provider_nodes(&self) -> Vec<MixProviderNode>;
    fn get_coco_nodes(&self) -> Vec<CocoNode>;
    fn route_from(&self) -> Vec<SphinxNode> {
        let mut layered_topology: HashMap<u64, Vec<MixNode>> = HashMap::new();
        for mix in self.get_mix_nodes() {
            let layer_nodes = layered_topology.entry(mix.layer).or_insert(Vec::new());
            layer_nodes.push(mix);
        }

        // TODO: assertion that num_layers is a sane number
        let num_layers = layered_topology.len() as u64;

        let route: Vec<_> = (1..=num_layers)
            .map(|layer| &layered_topology[&layer]) // for each layer
            .map(|nodes| nodes.choose(&mut rand::thread_rng()).unwrap()) // choose random node
            .collect();

        route
            .iter()
            .map(|mix| {
                let address_bytes = addressing::encoded_bytes_from_socket_address(mix.host.clone());
                let decoded_key_bytes =
                    base64::decode_config(&mix.pub_key, base64::URL_SAFE).unwrap();
                let mut key_bytes = [0; 32];
                key_bytes.copy_from_slice(&decoded_key_bytes[..]);
                let key = MontgomeryPoint(key_bytes);
                SphinxNode {
                    address: NodeAddressBytes::from_bytes(address_bytes),
                    pub_key: key,
                }
            })
            .collect()
    }
}
