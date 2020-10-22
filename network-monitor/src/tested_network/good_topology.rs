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

use crypto::asymmetric::{
    encryption::{self, PublicKey},
    identity,
};
use std::collections::HashMap;
use topology::{gateway, mix, NymTopology};

pub(crate) fn v4_mixnodes() -> Vec<mix::Node> {
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

pub(crate) fn v6_mixnodes() -> Vec<mix::Node> {
    let goodnode1 = mix::Node {
        location: "London".to_string(),
        host: "[2a01:7e00::f03c:92ff:fe16:3dc2]:1789".parse().unwrap(),
        pub_key: PublicKey::from_base58_string("EJHwrLafqygqctkBCntVZfUkMSDErGUStJjZniQoRoJr")
            .unwrap(),
        last_seen: 1600276206950298819,
        layer: 1,
        version: "0.8.0".to_string(),
    };

    let goodnode2 = mix::Node {
        location: "Frankfurt".to_string(),
        host: "[2a01:7e01::f03c:92ff:fe16:3d11]:1789".parse().unwrap(),
        pub_key: PublicKey::from_base58_string("BW7xskYvZyHt8rGFzsmG5bEQ9ViCYYxpFsEWDcNtSYvX")
            .unwrap(),
        last_seen: 1600276206950298819,
        layer: 2,
        version: "0.8.0".to_string(),
    };

    let goodnode3 = mix::Node {
        location: "London".to_string(),
        host: "[2a01:7e00::f03c:92ff:fe16:3d7b]:1789".parse().unwrap(),
        pub_key: PublicKey::from_base58_string("BqBGpP4YDH5fRDVKB97Ru7aq2Wbarb3SNfZL5LGaH83e")
            .unwrap(),
        layer: 3,
        last_seen: 1600276206950298819,
        version: "0.8.0".to_string(),
    };

    vec![goodnode1, goodnode2, goodnode3]
}

pub(crate) fn v4_gateway() -> gateway::Node {
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

pub(crate) fn v6_gateway() -> gateway::Node {
    gateway::Node {
        location: "unknown".to_string(),
        client_listener: "ws://[2a01:7e00::f03c:92ff:fe16:49f1]:9000".to_string(),
        mixnet_listener: "[2a01:7e00::f03c:92ff:fe16:49f1]:1789".parse().unwrap(),
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

/// Returns a new ipv4 NymTopology composed of known good nodes
pub(crate) fn new_v4() -> NymTopology {
    let mut layered_mixes = HashMap::new();

    for (i, node) in v4_mixnodes().into_iter().enumerate() {
        layered_mixes.insert((i + 1) as u8, vec![node]);
    }

    NymTopology::new(Vec::new(), layered_mixes, vec![v4_gateway()])
}

/// Returns a new ipv6 NymTopology composed of known good nodes
pub(crate) fn new_v6() -> NymTopology {
    let mut layered_mixes = HashMap::new();

    for (i, node) in v6_mixnodes().into_iter().enumerate() {
        layered_mixes.insert((i + 1) as u8, vec![node]);
    }

    NymTopology::new(Vec::new(), layered_mixes, vec![v6_gateway()])
}

#[cfg(test)]
mod good_topology_test {
    use super::*;

    // Returns a new topology of known good nodes, with one good node replaced with a test node
    fn new_v4_with_node(test_node: mix::Node) -> NymTopology {
        let mut topology = self::new_v4();
        topology.set_mixes_in_layer(test_node.layer as u8, vec![test_node]);
        topology
    }

    mod subbing_in_a_node_to_test {
        use super::*;

        #[test]
        fn returns_good_topology_with_test_node_in_desired_layer() {
            let topology = expected_topology_with_test_node();
            let expected_gateway_key = topology.gateways().first().unwrap().identity_key;
            let expected_layer_1_mixnode_pubkey =
                topology.mixes_in_layer(1)[0].pub_key.to_base58_string();
            let expected_layer_2_mixnode_pubkey =
                topology.mixes_in_layer(2)[0].pub_key.to_base58_string();
            let expected_layer_3_mixnode_pubkey =
                topology.mixes_in_layer(3)[0].pub_key.to_base58_string();
            let result = new_v4_with_node(test_node());
            let actual_gateway_key = result.gateways().first().unwrap().identity_key;
            let actual_layer_1_mixnode_pubkey =
                result.mixes_in_layer(1)[0].pub_key.to_base58_string();
            let actual_layer_2_mixnode_pubkey =
                result.mixes_in_layer(2)[0].pub_key.to_base58_string();
            let actual_layer_3_mixnode_pubkey =
                result.mixes_in_layer(3)[0].pub_key.to_base58_string();

            assert_eq!(expected_gateway_key, actual_gateway_key);
            assert_eq!(
                expected_layer_1_mixnode_pubkey,
                actual_layer_1_mixnode_pubkey
            );
            assert_eq!(
                expected_layer_2_mixnode_pubkey,
                actual_layer_2_mixnode_pubkey
            );
            assert_eq!(
                expected_layer_3_mixnode_pubkey,
                actual_layer_3_mixnode_pubkey
            );
        }
    }

    fn expected_topology_with_test_node() -> NymTopology {
        let mut mixes = HashMap::new();
        let mixnodes = v4_mixnodes();
        let mix1 = test_node(); // this is the one we will test
        let mix2 = mixnodes[1].clone();
        let mix3 = mixnodes[2].clone();

        mixes.insert(1, vec![mix1]);
        mixes.insert(2, vec![mix2]);
        mixes.insert(3, vec![mix3]);
        NymTopology::new(vec![], mixes, vec![v4_gateway()])
    }

    fn test_node() -> mix::Node {
        mix::Node {
            location: "Thunder Bay".to_string(),
            host: "1.2.3.4:1234".parse().unwrap(),
            pub_key: encryption::PublicKey::from_base58_string(
                "9fX1rMaQdBEzjuv6kT7oyPfEabt73QTM5cfuQ9kaxrRQ",
            )
            .unwrap(),
            layer: 1,
            last_seen: 1234,
            version: "0.8.1".to_string(),
        }
    }
}
