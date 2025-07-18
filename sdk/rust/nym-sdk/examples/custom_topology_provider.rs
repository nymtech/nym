// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_topology::provider_trait::{async_trait, ToTopologyMetadata, TopologyProvider};
use nym_topology::NymTopology;
use url::Url;

struct MyTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
}

impl MyTopologyProvider {
    fn new(nym_api_url: Url) -> MyTopologyProvider {
        MyTopologyProvider {
            validator_client: nym_validator_client::client::NymApiClient::new(nym_api_url),
        }
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

        let mut base_topology = NymTopology::new(metadata, rewarded_set, Vec::new());

        // in our topology provider only use mixnodes that have node_id divisible by 3
        // and has exactly 100 performance score
        // why? because this is just an example to showcase arbitrary uses and capabilities of this trait
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

#[async_trait]
impl TopologyProvider for MyTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        Some(self.get_topology().await)
    }
}

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();
    let my_topology_provider = MyTopologyProvider::new(nym_api);

    // Passing no config makes the client fire up an ephemeral session and figure things out on its own
    let mut client = mixnet::MixnetClientBuilder::new_ephemeral()
        .custom_topology_provider(Box::new(my_topology_provider))
        .build()
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

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
