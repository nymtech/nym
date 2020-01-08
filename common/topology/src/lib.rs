use addressing;
use curve25519_dalek::montgomery::MontgomeryPoint;
use rand::seq::SliceRandom;
use sphinx::route::Node as SphinxNode;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::cmp::max;

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
                return Err(NymTopologyError::InvalidMixLayerError)
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
        let layered_topology = self.make_layered_topology()?;
        let num_layers = layered_topology.len();
        let route: Vec<_> = (1..=num_layers as u64)
            .map(|layer| &layered_topology[&layer]) // for each layer
            .map(|nodes| nodes.choose(&mut rand::thread_rng()).unwrap()) // choose random node
            .collect();

        Ok(route
            .iter()
            .map(|mix| {
                let address_bytes = addressing::encoded_bytes_from_socket_address(mix.host.clone());
                let decoded_key_bytes =
                    base64::decode_config(&mix.pub_key, base64::URL_SAFE).unwrap();
                let mut key_bytes = [0; 32];
                key_bytes.copy_from_slice(&decoded_key_bytes[..]);
                let key = MontgomeryPoint(key_bytes);
                SphinxNode {
                    address: address_bytes,
                    pub_key: key,
                }
            })
            .collect())
    }
    }
}
