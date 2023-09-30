// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();

    // We can group on something which is to a first approximation a continent.
    let group_by = mixnet::GroupBy::CountryGroup(mixnet::CountryGroup::Europe);

    // ... or on a nym-address. This means we use the geo location of the gateway that the
    // nym-address is connected to.
    //let group_by = GroupBy::NymAddress("id.enc@gateway".parse().unwrap());

    let geo_topology_provider = mixnet::GeoAwareTopologyProvider::new(
        vec![nym_api],
        // We filter on the version of the mixnodes. Be prepared to manually update
        // this to keep this example working, as we can't (currently) fetch to current
        // latest version.
        "1.1.31".to_string(),
        group_by,
    );

    // Passing no config makes the client fire up an ephemeral session and figure things out on its own
    let mut client = mixnet::MixnetClientBuilder::new_ephemeral()
        .custom_topology_provider(Box::new(geo_topology_provider))
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
