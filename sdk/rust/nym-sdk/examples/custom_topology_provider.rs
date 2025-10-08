// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sdk::{mixnet, DebugConfig};
use nym_topology::provider_trait::{async_trait, ToTopologyMetadata, TopologyProvider};
use nym_topology::{
    CachedEpochRewardedSet, EntryDetails, EpochRewardedSet, HardcodedTopologyProvider, NymTopology,
    NymTopologyMetadata, RoutingNode, SupportedRoles,
};
use nym_validator_client::nym_api::NymApiClientExt;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use url::Url;
//
// struct MyTopologyProvider {
//     validator_client: nym_http_api_client::Client,
// }
//
// impl MyTopologyProvider {
//     fn new(nym_api_url: Url) -> MyTopologyProvider {
//         let validator_client = nym_http_api_client::Client::builder(nym_api_url)
//             .expect("Failed to create API client builder")
//             .build()
//             .expect("Failed to build API client");
//
//         MyTopologyProvider { validator_client }
//     }
//
//     async fn get_topology(&self) -> NymTopology {
//         let rewarded_set = self
//             .validator_client
//             .get_current_rewarded_set()
//             .await
//             .unwrap();
//
//         let mixnodes_response = self
//             .validator_client
//             .get_all_basic_active_mixing_assigned_nodes_with_metadata()
//             .await
//             .unwrap();
//
//         let metadata = mixnodes_response.metadata.to_topology_metadata();
//
//         let epoch_rewarded_set: EpochRewardedSet = rewarded_set.into();
//         let mut base_topology = NymTopology::new(metadata, epoch_rewarded_set, Vec::new());
//
//         // in our topology provider only use mixnodes that have node_id divisible by 3
//         // and has exactly 100 performance score
//         // why? because this is just an example to showcase arbitrary uses and capabilities of this trait
//         let filtered_mixnodes = mixnodes_response
//             .nodes
//             .into_iter()
//             .filter(|mix| mix.node_id % 3 == 0 && mix.performance.is_hundred())
//             .collect::<Vec<_>>();
//
//         let gateways = self
//             .validator_client
//             .get_all_basic_entry_assigned_nodes_with_metadata()
//             .await
//             .unwrap()
//             .nodes;
//
//         base_topology.add_skimmed_nodes(&filtered_mixnodes);
//         base_topology.add_skimmed_nodes(&gateways);
//         base_topology
//     }
// }
//
// #[async_trait]
// impl TopologyProvider for MyTopologyProvider {
//     // this will be manually refreshed on a timer specified inside mixnet client config
//     async fn get_new_topology(&mut self) -> Option<NymTopology> {
//         Some(self.get_topology().await)
//     }
// }

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

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
