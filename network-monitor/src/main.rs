use std::sync::Arc;

use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use network::{good_topology::gateway, AckSender, MixnetSender};
use nymsphinx::addressing::clients::Recipient;

use std::time;

use gateway_client::GatewayClient;
use topology::gateway;

mod network;

fn main() {
    println!("Network monitor starting...");

    // Set up topology
    let directory_uri = "https://qa-directory.nymtech.net".to_string();
    let good_topology = network::good_topology::new();
    let gateway = gateway();

    // Channels for task communication
    let (ack_sender, ack_receiver) = mpsc::unbounded();
    let (mixnet_sender, mixnet_receiver) = mpsc::unbounded();

    // Generate a new set of identity keys. These are ephemeral, and change on each run.
    let identity_keypair = identity::KeyPair::new();
    let encryption_keypair = encryption::KeyPair::new();

    // We need our own address as a Recipient so we can send ourselves test packets
    let self_address = Recipient::new(
        identity_keypair.public_key().clone(),
        encryption_keypair.public_key().clone(),
        gateway.identity_key,
    );

    let gateway_client = new_gateway_client(gateway, identity_keypair, ack_sender, mixnet_sender);

    // Construct a new `network::Monitor`.
    let config = network::MonitorConfig {
        ack_receiver,
        directory_uri: directory_uri.clone(),
        gateway_client,
        good_topology,
        self_address,
    };
    let mut network_monitor = network::Monitor::new(config);

    println!("* directory server: {}", directory_uri);
    network_monitor.run(mixnet_receiver, encryption_keypair);
}

/// Construct a new gateway client.
pub fn new_gateway_client(
    gateway: gateway::Node,
    identity_keypair: identity::KeyPair,
    ack_sender: AckSender,
    mixnet_messages_sender: MixnetSender,
) -> GatewayClient {
    let timeout = time::Duration::from_millis(500);
    let identity_arc = Arc::new(identity_keypair);

    gateway_client::GatewayClient::new(
        gateway.client_listener,
        identity_arc,
        gateway.identity_key,
        None,
        mixnet_messages_sender,
        ack_sender,
        timeout,
    )
}
