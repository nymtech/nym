use std::collections::HashMap;

use crypto::asymmetric::{
    encryption::{self, PublicKey},
    identity,
};
use topology::{gateway, mix, NymTopology};

pub(crate) fn mixnodes() -> Vec<mix::Node> {
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

    vec![goodnode1, goodnode2, goodnode3]
}

pub(crate) fn gateway() -> gateway::Node {
    gateway::Node {
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
    }
}

pub(crate) fn new() -> NymTopology {
    let mut layered_mixes = HashMap::new();

    for (i, node) in mixnodes().iter().enumerate() {
        layered_mixes.insert((i + 1) as u8, vec![node.clone()]);
    }

    NymTopology::new(vec![], layered_mixes, vec![gateway()])
}
