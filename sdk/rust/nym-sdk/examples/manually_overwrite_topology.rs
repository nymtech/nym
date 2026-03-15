// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Manually overwriting the network topology at runtime.
//!
//! Connects an ephemeral client, then replaces its topology with a
//! hand-picked set of mix nodes while keeping the original gateways.
//! All subsequent traffic is routed through these specific nodes.
//!
//! Run with: cargo run --example manually_overwrite_topology

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_topology::{NymTopology, NymTopologyMetadata, RoutingNode, SupportedRoles};

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Connect an ephemeral client and grab the current topology.
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let starting_topology = client.read_current_route_provider().await.unwrap().clone();

    // Step 2: Define a custom set of hardcoded mix nodes.
    let nodes = vec![
        RoutingNode {
            node_id: 63,
            mix_host: "172.105.92.48:1789".parse().unwrap(),
            entry: None,
            identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK"
                .parse()
                .unwrap(),
            sphinx_key: "CBmYewWf43iarBq349KhbfYMc9ys2ebXWd4Vp4CLQ5Rq"
                .parse()
                .unwrap(),
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: false,
                mixnet_exit: false,
            },
        },
        RoutingNode {
            node_id: 23,
            mix_host: "178.79.143.65:1789".parse().unwrap(),
            entry: None,
            identity_key: "4Yr4qmEHd9sgsuQ83191FR2hD88RfsbMmB4tzhhZWriz"
                .parse()
                .unwrap(),
            sphinx_key: "8ndjk5oZ6HxUZNScLJJ7hk39XtUqGexdKgW7hSX6kpWG"
                .parse()
                .unwrap(),
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: false,
                mixnet_exit: false,
            },
        },
        RoutingNode {
            node_id: 66,
            mix_host: "139.162.247.97:1789".parse().unwrap(),
            entry: None,
            identity_key: "66UngapebhJRni3Nj52EW1qcNsWYiuonjkWJzHFsmyYY"
                .parse()
                .unwrap(),
            sphinx_key: "7KyZh8Z8KxuVunqytAJ2eXFuZkCS7BLTZSzujHJZsGa2"
                .parse()
                .unwrap(),
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: false,
                mixnet_exit: false,
            },
        },
    ];

    // Step 3: Build a custom topology using these nodes plus the original gateways.
    // Inject our custom nodes into the rewarded set so the client will use them.
    let mut rewarded_set = starting_topology.topology.rewarded_set().clone();
    rewarded_set.layer1.insert(nodes[0].node_id);
    rewarded_set.layer2.insert(nodes[1].node_id);
    rewarded_set.layer3.insert(nodes[2].node_id);

    // Keep the original gateways so we can still send to ourselves.
    let gateways = starting_topology.topology.entry_capable_nodes();

    // In production, obtain valid metadata (especially the key rotation ID).
    let metadata = NymTopologyMetadata::new(u32::MAX, 123, time::OffsetDateTime::now_utc());

    let mut custom_topology = NymTopology::new(metadata, rewarded_set, Vec::new());
    custom_topology.add_routing_nodes(nodes);
    custom_topology.add_routing_nodes(gateways);

    // Step 4: Apply the custom topology. All subsequent traffic goes
    // through these specific nodes.
    client.manually_overwrite_topology(custom_topology).await;

    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Step 5: Send a message to ourselves through the custom topology.
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    println!("Waiting for message (ctrl-c to exit)");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
