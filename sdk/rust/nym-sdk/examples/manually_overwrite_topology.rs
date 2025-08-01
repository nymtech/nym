// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_topology::{NymTopology, NymTopologyMetadata, RoutingNode, SupportedRoles};

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Passing no config makes the client fire up an ephemeral session and figure shit out on its own
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let starting_topology = client.read_current_route_provider().await.unwrap().clone();

    // but we don't like our default topology, we want to use only those very specific, hardcoded, nodes:
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

    // make sure our custom nodes are in the fake rewarded set (so they'd be used by default by the client)
    let mut rewarded_set = starting_topology.topology.rewarded_set().clone();
    rewarded_set.layer1.insert(nodes[0].node_id);
    rewarded_set.layer2.insert(nodes[1].node_id);
    rewarded_set.layer3.insert(nodes[2].node_id);

    // but we like the available gateways, so keep using them!
    // (we like them because the author of this example is too lazy to use the same hardcoded gateway
    // during client initialisation to make sure we are able to send to ourselves : )  )
    let gateways = starting_topology.topology.entry_capable_nodes();

    // you should have obtained valid metadata information, in particular the key rotation ID!
    let metadata = NymTopologyMetadata::new(u32::MAX, 123, time::OffsetDateTime::now_utc());

    let mut custom_topology = NymTopology::new(metadata, rewarded_set, Vec::new());
    custom_topology.add_routing_nodes(nodes);
    custom_topology.add_routing_nodes(gateways);

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
