// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_topology::mix::Layer;
use nym_topology::{mix, NymTopology};
use std::collections::BTreeMap;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Passing no config makes the client fire up an ephemeral session and figure shit out on its own
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let starting_topology = client.read_current_topology().await.unwrap();

    // but we don't like our default topology, we want to use only those very specific, hardcoded, nodes:
    let mut mixnodes = BTreeMap::new();
    mixnodes.insert(
        1,
        vec![mix::Node {
            mix_id: 63,
            owner: "n1k52k5n45cqt5qpjh8tcwmgqm0wkt355yy0g5vu".to_string(),
            host: "172.105.92.48".parse().unwrap(),
            mix_hosts: vec!["172.105.92.48:1789".parse().unwrap()],
            identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK"
                .parse()
                .unwrap(),
            sphinx_key: "CBmYewWf43iarBq349KhbfYMc9ys2ebXWd4Vp4CLQ5Rq"
                .parse()
                .unwrap(),
            layer: Layer::One,
            version: "1.1.0".into(),
        }],
    );
    mixnodes.insert(
        2,
        vec![mix::Node {
            mix_id: 23,
            owner: "n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47".to_string(),
            host: "178.79.143.65".parse().unwrap(),
            mix_hosts: vec!["178.79.143.65:1789".parse().unwrap()],
            identity_key: "4Yr4qmEHd9sgsuQ83191FR2hD88RfsbMmB4tzhhZWriz"
                .parse()
                .unwrap(),
            sphinx_key: "8ndjk5oZ6HxUZNScLJJ7hk39XtUqGexdKgW7hSX6kpWG"
                .parse()
                .unwrap(),
            layer: Layer::Two,
            version: "1.1.0".into(),
        }],
    );
    mixnodes.insert(
        3,
        vec![mix::Node {
            mix_id: 66,
            owner: "n1ae2pjd7q9p0dea65pqkvcm4x9s264v4fktpyru".to_string(),
            host: "139.162.247.97".parse().unwrap(),
            mix_hosts: vec!["139.162.247.97:1789".parse().unwrap()],
            identity_key: "66UngapebhJRni3Nj52EW1qcNsWYiuonjkWJzHFsmyYY"
                .parse()
                .unwrap(),
            sphinx_key: "7KyZh8Z8KxuVunqytAJ2eXFuZkCS7BLTZSzujHJZsGa2"
                .parse()
                .unwrap(),
            layer: Layer::Three,
            version: "1.1.0".into(),
        }],
    );

    // but we like the available gateways, so keep using them!
    // (we like them because the author of this example is too lazy to use the same hardcoded gateway
    // during client initialisation to make sure we are able to send to ourselves : )  )
    let custom_topology = NymTopology::new(mixnodes, starting_topology.gateways().to_vec());

    client.manually_overwrite_topology(custom_topology).await;

    // and everything we send now should only ever go via those nodes

    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message through the mixnet to ourselves
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    println!("Waiting for message (ctrl-c to exit)");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
