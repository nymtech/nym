// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet;
use nym_topology::provider_trait::{async_trait, TopologyProvider};
use nym_topology::{nym_topology_from_detailed, NymTopology};
use url::Url;

struct MyTopologyProvider {
    validator_client: validator_client::client::NymApiClient,
}

impl MyTopologyProvider {
    fn new(nym_api_url: Url) -> MyTopologyProvider {
        MyTopologyProvider {
            validator_client: validator_client::client::NymApiClient::new(nym_api_url),
        }
    }

    async fn get_topology(&self) -> NymTopology {
        let mixnodes = self
            .validator_client
            .get_cached_active_mixnodes()
            .await
            .unwrap();

        // in our topology provider only use mixnodes that have mix_id divisible by 3
        // and have more than 100k nym (i.e. 100'000'000'000 unym) in stake
        // why? because this is just an example to showcase arbitrary uses and capabilities of this trait
        let filtered_mixnodes = mixnodes
            .into_iter()
            .filter(|mix| {
                mix.mix_id() % 3 == 0 && mix.total_stake() > "100000000000".parse().unwrap()
            })
            .collect::<Vec<_>>();

        let gateways = self.validator_client.get_cached_gateways().await.unwrap();

        nym_topology_from_detailed(filtered_mixnodes, gateways)
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
    nym_bin_common::logging::setup_logging();

    let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();
    let my_topology_provider = MyTopologyProvider::new(nym_api);

    // Passing no config makes the client fire up an ephemeral session and figure shit out on its own
    let mut client = mixnet::MixnetClientBuilder::new()
        .custom_topology_provider(Box::new(my_topology_provider))
        .build::<mixnet::EmptyReplyStorage>()
        .await
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message through the mixnet to ourselves
    client.send_str(*our_address, "hello there").await;

    println!("Waiting for message (ctrl-c to exit)");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
