use addressing;
use curve25519_dalek::montgomery::MontgomeryPoint;
use itertools::Itertools;
use rand::seq::IteratorRandom;
use sphinx::route::{Node as SphinxNode, NodeAddressBytes};
use std::cmp::max;
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

impl Into<SphinxNode> for MixNode {
    fn into(self) -> SphinxNode {
        let address_bytes = addressing::encoded_bytes_from_socket_address(self.host);
        let key_bytes = self.get_pub_key_bytes();
        let key = MontgomeryPoint(key_bytes);

        SphinxNode::new(NodeAddressBytes::from_bytes(address_bytes), key)
    }
}

impl MixNode {
    pub fn get_pub_key_bytes(&self) -> [u8; 32] {
        let decoded_key_bytes = base64::decode_config(&self.pub_key, base64::URL_SAFE).unwrap();
        let mut key_bytes = [0; 32];
        key_bytes.copy_from_slice(&decoded_key_bytes[..]);
        key_bytes
    }
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

impl Into<SphinxNode> for MixProviderNode {
    fn into(self) -> SphinxNode {
        let address_bytes = addressing::encoded_bytes_from_socket_address(self.mixnet_listener);
        let key_bytes = self.get_pub_key_bytes();
        let key = MontgomeryPoint(key_bytes);

        SphinxNode::new(NodeAddressBytes::from_bytes(address_bytes), key)
    }
}

impl MixProviderNode {
    pub fn get_pub_key_bytes(&self) -> [u8; 32] {
        let decoded_key_bytes = base64::decode_config(&self.pub_key, base64::URL_SAFE).unwrap();
        let mut key_bytes = [0; 32];
        key_bytes.copy_from_slice(&decoded_key_bytes[..]);
        key_bytes
    }
}

#[derive(Debug)]
pub struct CocoNode {
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

#[derive(Debug)]
pub enum NymTopologyError {
    InvalidMixLayerError,
    MissingLayerError(Vec<u64>),
}

pub trait NymTopology {
    fn new(directory_server: String) -> Self;
    fn get_mix_nodes(&self) -> Vec<MixNode>;
    fn get_mix_provider_nodes(&self) -> Vec<MixProviderNode>;
    fn get_coco_nodes(&self) -> Vec<CocoNode>;
    fn make_layered_topology(&self) -> Result<HashMap<u64, Vec<MixNode>>, NymTopologyError> {
        let mut layered_topology: HashMap<u64, Vec<MixNode>> = HashMap::new();
        let mut highest_layer = 0;
        for mix in self.get_mix_nodes() {
            // we need to have extra space for provider
            if mix.layer > sphinx::constants::MAX_PATH_LENGTH as u64 {
                return Err(NymTopologyError::InvalidMixLayerError);
            }
            highest_layer = max(highest_layer, mix.layer);

            let layer_nodes = layered_topology.entry(mix.layer).or_insert(Vec::new());
            layer_nodes.push(mix);
        }

        // verify the topology - make sure there are no gaps and there is at least one node per layer
        let mut missing_layers = Vec::new();
        for layer in 1..=highest_layer {
            if !layered_topology.contains_key(&layer) {
                missing_layers.push(layer);
            }
            if layered_topology[&layer].len() == 0 {
                missing_layers.push(layer);
            }
        }

        if missing_layers.len() > 0 {
            return Err(NymTopologyError::MissingLayerError(missing_layers));
        }

        Ok(layered_topology)
    }
    fn mix_route(&self) -> Result<Vec<SphinxNode>, NymTopologyError> {
        let mut layered_topology = self.make_layered_topology()?;
        let num_layers = layered_topology.len();
        let route = (1..=num_layers as u64)
            .map(|layer| layered_topology.remove(&layer).unwrap()) // for each layer
            .map(|nodes| nodes.into_iter().choose(&mut rand::thread_rng()).unwrap()) // choose random node
            .map(|random_node| random_node.into()) // and convert it into sphinx specific node format
            .collect();

        Ok(route)
    }

    // sets a route to specific provider
    fn route_to(&self, provider_node: SphinxNode) -> Result<Vec<SphinxNode>, NymTopologyError> {
        Ok(self
            .mix_route()?
            .into_iter()
            .chain(std::iter::once(provider_node))
            .collect())
    }

    fn all_paths(&self) -> Result<Vec<Vec<SphinxNode>>, NymTopologyError> {
        let mut layered_topology = self.make_layered_topology()?;
        let providers = self.get_mix_provider_nodes();

        let sorted_layers: Vec<Vec<SphinxNode>> = (1..=layered_topology.len() as u64)
            .map(|layer| layered_topology.remove(&layer).unwrap()) // get all nodes per layer
            .map(|layer_nodes| layer_nodes.into_iter().map(|node| node.into()).collect()) // convert them into 'proper' sphinx nodes
            .chain(std::iter::once(
                providers.into_iter().map(|node| node.into()).collect(),
            )) // append all providers to the end
            .collect();

        let all_paths = sorted_layers
            .into_iter()
            .multi_cartesian_product() // create all possible paths through that
            .collect();

        Ok(all_paths)
    }
}

// TODO: tests...
