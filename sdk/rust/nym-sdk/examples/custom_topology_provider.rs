// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sdk::{mixnet};
use nym_topology::{
    CachedEpochRewardedSet, EntryDetails, HardcodedTopologyProvider, NymTopology,
    NymTopologyMetadata, RoutingNode, SupportedRoles,
};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_no_otel_logger().expect("failed to setup logging");

    // let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();
    // let my_topology_provider = MyTopologyProvider::new(nym_api);

    let topology_metadata = NymTopologyMetadata::new(0, 0, OffsetDateTime::now_utc());
    let mut rewarded_set = CachedEpochRewardedSet::default();
    rewarded_set.entry_gateways.insert(1);
    rewarded_set.layer1.insert(1);
    rewarded_set.layer2.insert(1);
    rewarded_set.layer3.insert(1);

    let nodes = vec![RoutingNode {
        node_id: 1,
        mix_host: "127.0.0.1:1789".parse().unwrap(),
        entry: Some(EntryDetails {
            ip_addresses: vec!["127.0.0.1".parse().unwrap()],
            clients_ws_port: 9000,
            hostname: None,
            clients_wss_port: None,
        }),
        identity_key: "PUT IDENTITY KEY HERE"
            .parse()
            .unwrap(),
        sphinx_key: "PUT SPHINX KEY HERE"
            .parse()
            .unwrap(),
        supported_roles: SupportedRoles {
            mixnode: true,
            mixnet_entry: true,
            mixnet_exit: true,
        },
    }];

    let topology_provider =
        HardcodedTopologyProvider::new(NymTopology::new(topology_metadata, rewarded_set, nodes));

    // Passing no config makes the client fire up an ephemeral session and figure things out on its own
    let mut client = mixnet::MixnetClientBuilder::new_ephemeral()
        .custom_topology_provider(Box::new(topology_provider))
        .build()
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

    let our_address = *client.nym_address();
    println!("Our client nym address is: {our_address}");

    let sender = client.split_sender();

    const MAX_MESSAGES: usize = 100;

    // receiving task
    let receiving_task_handle = tokio::spawn(async move {
        let mut received_count = 0;
        while let Some(received) = client.next().await {
            received_count += 1;
            println!(
                "{received_count}: received: {}",
                String::from_utf8_lossy(&received.message)
            );
            if received_count >= MAX_MESSAGES {
                break;
            }
        }
        client.disconnect().await;
    });

    // sending task
    let sending_task_handle = tokio::spawn(async move {
        loop {
            if sender
                .send_plain_message(our_address, "hello there")
                .await
                .is_err()
            {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    });

    // wait for both tasks to be done
    println!("waiting for shutdown");
    sending_task_handle.await.unwrap();
    receiving_task_handle.await.unwrap();
}
