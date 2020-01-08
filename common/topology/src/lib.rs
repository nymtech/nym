use addressing;
use curve25519_dalek::montgomery::MontgomeryPoint;
use directory_client::presence::{MixNodePresence, Topology};
use directory_client::requests::presence_topology_get::PresenceTopologyGetRequester;
use directory_client::DirectoryClient;
use rand::seq::SliceRandom;
use sphinx::route::Node as SphinxNode;
use std::collections::HashMap;

pub fn get_topology(directory_server: String) -> Topology {
    println!("Using directory server: {:?}", directory_server);
    let directory_config = directory_client::Config {
        base_url: directory_server,
    };
    let directory = directory_client::Client::new(directory_config);

    let topology = directory
        .presence_topology
        .get()
        .expect("Failed to retrieve network topology.");
    topology
}

pub fn route_from(topology: &Topology) -> Vec<SphinxNode> {
    let mut layered_topology: HashMap<u64, Vec<MixNodePresence>> = HashMap::new();
    let mixes = topology.mix_nodes.iter();
    for mix in mixes {
        let layer_nodes = layered_topology.entry(mix.layer).or_insert(Vec::new());
        layer_nodes.push(mix.clone());
    }

    let num_layers = layered_topology.len() as u64;
    let mut route = vec![];

    for x in 1..=num_layers {
        let nodes = &layered_topology[&x];
        let the_node = nodes.choose(&mut rand::thread_rng()).unwrap();
        route.push(the_node);
    }

    route
        .iter()
        .map(|mix| {
            let address_bytes =
                addressing::encoded_bytes_from_socket_address(mix.host.clone().parse().unwrap());
            let decoded_key_bytes = base64::decode_config(&mix.pub_key, base64::URL_SAFE).unwrap();
            let mut key_bytes = [0; 32];
            key_bytes.copy_from_slice(&decoded_key_bytes[..]);
            let key = MontgomeryPoint(key_bytes);
            SphinxNode {
                address: address_bytes,
                pub_key: key,
            }
        })
        .collect()
}
