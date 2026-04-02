// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Custom topology provider that filters mix nodes from the Nym API.
//!
//! Implements `TopologyProvider` to fetch the network topology and apply
//! custom filtering (here: only nodes with `node_id % 3 == 0` and a
//! perfect performance score). Shows how to plug alternative topology
//! sources into the client builder.
//!
//! Run with: cargo run --example custom_topology_provider

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_topology::provider_trait::{async_trait, ToTopologyMetadata, TopologyProvider};
use nym_topology::{EpochRewardedSet, NymTopology};
use nym_validator_client::nym_api::NymApiClientExt;
use url::Url;

// Define a custom topology provider.
// It fetches topology from the Nym API and applies arbitrary filtering.
struct MyTopologyProvider {
    validator_client: nym_http_api_client::Client,
}

impl MyTopologyProvider {
    fn new(nym_api_url: Url) -> MyTopologyProvider {
        let validator_client = nym_http_api_client::Client::builder(nym_api_url)
            .expect("Failed to create API client builder")
            .build()
            .expect("Failed to build API client");

        MyTopologyProvider { validator_client }
    }

    async fn get_topology(&self) -> NymTopology {
        let rewarded_set = self
            .validator_client
            .get_current_rewarded_set()
            .await
            .unwrap();

        let mixnodes_response = self
            .validator_client
            .get_all_basic_active_mixing_assigned_nodes_with_metadata()
            .await
            .unwrap();

        let metadata = mixnodes_response.metadata.to_topology_metadata();

        let epoch_rewarded_set: EpochRewardedSet = rewarded_set.into();
        let mut base_topology = NymTopology::new(metadata, epoch_rewarded_set, Vec::new());

        // Custom filter: only mix nodes with node_id divisible by 3
        // and a perfect performance score.
        let filtered_mixnodes = mixnodes_response
            .nodes
            .into_iter()
            .filter(|mix| mix.node_id % 3 == 0 && mix.performance.is_hundred())
            .collect::<Vec<_>>();

        let gateways = self
            .validator_client
            .get_all_basic_entry_assigned_nodes_with_metadata()
            .await
            .unwrap()
            .nodes;

        base_topology.add_skimmed_nodes(&filtered_mixnodes);
        base_topology.add_skimmed_nodes(&gateways);
        base_topology
    }
}

// Implement the TopologyProvider trait.
// The client refreshes topology on a timer using this method.
#[async_trait]
impl TopologyProvider for MyTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        Some(self.get_topology().await)
    }
}

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Create the provider and pass it to the client builder.
    let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();
    let my_topology_provider = MyTopologyProvider::new(nym_api);

    let mut client = mixnet::MixnetClientBuilder::new_ephemeral()
        .custom_topology_provider(Box::new(my_topology_provider))
        .build()
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message to ourselves using the custom topology.
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    println!("Waiting for message (ctrl-c to exit)");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
