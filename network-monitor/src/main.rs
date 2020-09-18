use std::collections::HashMap;

use crypto::asymmetric::{
    encryption::{self, PublicKey},
    identity,
};
use topology::{gateway, mix, NymTopology};

mod network;

fn main() {
    let websocket_uri = "ws://localhost:1977";
    let directory_uri = "https://directory.nymtech.net";
    let good_topology = construct_topology_from_known_nodes();

    println!("Starting network monitor:");
    let network_monitor = network::Monitor::new(directory_uri, good_topology, websocket_uri);
    network_monitor.run();
}

fn construct_topology_from_known_nodes() -> NymTopology {
    let goodnode1 = mix::Node {
        location: "London".to_string(),
        host: "213.52.129.218:1789".parse().unwrap(),
        pub_key: PublicKey::from_base58_string("EJHwrLafqygqctkBCntVZfUkMSDErGUStJjZniQoRoJr")
            .unwrap(),
        last_seen: 1600276206950298819,
        layer: 1,
        version: "0.8.0".to_string(),
    };

    let goodnode2 = mix::Node {
        location: "Frankfurt".to_string(),
        host: "172.104.244.117:1789".parse().unwrap(),
        pub_key: PublicKey::from_base58_string("BW7xskYvZyHt8rGFzsmG5bEQ9ViCYYxpFsEWDcNtSYvX")
            .unwrap(),
        last_seen: 1600276206950298819,
        layer: 2,
        version: "0.8.0".to_string(),
    };

    let goodnode3 = mix::Node {
        location: "London".to_string(),
        host: "178.79.136.231:1789".parse().unwrap(),
        pub_key: PublicKey::from_base58_string("BqBGpP4YDH5fRDVKB97Ru7aq2Wbarb3SNfZL5LGaH83e")
            .unwrap(),
        layer: 3,
        last_seen: 1600276206950298819,
        version: "0.8.0".to_string(),
    };

    let mut layered_mixes = HashMap::new();
    layered_mixes.insert(1, vec![goodnode1]);
    layered_mixes.insert(2, vec![goodnode2]);
    layered_mixes.insert(3, vec![goodnode3]);

    let good_gateway = gateway::Node {
        location: "unknown".to_string(),
        client_listener: "ws://139.162.246.48:9000".to_string(),
        mixnet_listener: "139.162.246.48:1789".parse().unwrap(),
        identity_key: identity::PublicKey::from_base58_string(
            "D6YaMzLSY7mANtSQRKXsmMZpqgqiVkeiagKM4V4oFPFr",
        )
        .unwrap(),
        sphinx_key: encryption::PublicKey::from_base58_string(
            "6snGVMCatcTnvjGPaf8Ye7kCnVn6ThEDdCs4TZ7DbDVj",
        )
        .unwrap(),
        last_seen: 1600424297774836793,
        version: "0.8.0".to_string(),
    };

    NymTopology::new(vec![], layered_mixes, vec![good_gateway])
}
