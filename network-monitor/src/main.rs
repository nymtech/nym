use std::sync::Arc;

use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use network::good_topology::gateway;
use nymsphinx::addressing::clients::Recipient;
use rand::rngs::OsRng;

mod network;

const DEFAULT_RNG: OsRng = OsRng;

fn main() {
    // Set up top-level info.
    let directory_uri = "https://directory.nymtech.net".to_string();
    let good_topology = network::good_topology::new();
    let (ack_sender, ack_receiver) = mpsc::unbounded();
    let (mixnet_sender, mixnet_receiver) = mpsc::unbounded();
    let local_identity = Arc::new(identity::KeyPair::new_with_rng(&mut DEFAULT_RNG));
    let gateway = gateway();

    let client_encryption_key = encryption::KeyPair::new().public_key().to_owned();
    let client_identity = local_identity.public_key().clone();
    let gateway_identity = gateway.identity_key;
    let self_address = Recipient::new(client_identity, client_encryption_key, gateway_identity);

    // inject the info and start things up
    println!("Starting network monitor...");
    let gateway_client =
        network::clients::new_gateway_client(gateway, local_identity, ack_sender, mixnet_sender);

    let config = network::Config {
        ack_receiver,
        directory_uri,
        gateway_client,
        good_topology,
        mixnet_receiver,
        self_address,
    };

    let mut network_monitor = network::Monitor::new(config);
    network_monitor.run();
}
