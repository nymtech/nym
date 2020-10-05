// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::monitor::MixnetReceiver;
use crate::test_run::{TestRunUpdateReceiver, TestRunUpdateSender};
use crypto::asymmetric::{encryption, identity};
use directory_client::DirectoryClient;
use futures::channel::mpsc;
use gateway_client::GatewayClient;
use monitor::{AckSender, MixnetSender, Monitor};
use notifications::Notifier;
use nymsphinx::addressing::clients::Recipient;
use packet_sender::PacketSender;
use rand::rngs::OsRng;
use std::sync::Arc;
use std::time;
use topology::{gateway, NymTopology};

mod chunker;
mod good_topology;
mod monitor;
mod notifications;
mod packet_sender;
mod test_packet;
mod test_run;

pub(crate) type DefRng = OsRng;
pub(crate) const DEFAULT_RNG: DefRng = OsRng;

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        // .filter_module("hyper", log::LevelFilter::Warn)
        // .filter_module("tokio_reactor", log::LevelFilter::Warn)
        // .filter_module("reqwest", log::LevelFilter::Warn)
        // .filter_module("mio", log::LevelFilter::Warn)
        // .filter_module("want", log::LevelFilter::Warn)
        // .filter_module("sled", log::LevelFilter::Warn)
        // .filter_module("tungstenite", log::LevelFilter::Warn)
        // .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .init();
}

#[tokio::main]
async fn main() {
    println!("Network monitor starting...");
    setup_logging();

    // Set up topology
    let directory_uri = "https://qa-directory.nymtech.net";
    println!("* directory server: {}", directory_uri);
    let good_topology = good_topology::new();
    let gateway = good_topology::gateway();
    println!("* gateway: {}", gateway.identity_key.to_base58_string());

    // Channels for task communication
    let (ack_sender, _ack_receiver) = mpsc::unbounded();
    let (mixnet_sender, mixnet_receiver) = mpsc::unbounded();
    let (test_run_sender, test_run_receiver) = mpsc::unbounded();

    // Generate a new set of identity keys. These are ephemeral, and change on each run.
    // JS: do they? or rather should they?
    let identity_keypair = identity::KeyPair::new();
    let encryption_keypair = encryption::KeyPair::new();

    // We need our own address as a Recipient so we can send ourselves test packets
    let self_address = Recipient::new(
        identity_keypair.public_key().clone(),
        encryption_keypair.public_key().clone(),
        gateway.identity_key,
    );

    let directory_client = new_directory_client(directory_uri);

    let mut network_monitor = Monitor::new();

    let notifier = new_notifier(
        encryption_keypair,
        Arc::clone(&directory_client),
        mixnet_receiver,
        test_run_receiver,
    );

    let gateway_client = new_gateway_client(gateway, identity_keypair, ack_sender, mixnet_sender);

    let packet_sender = new_packet_sender(
        directory_client,
        good_topology,
        self_address,
        gateway_client,
        test_run_sender,
    );

    network_monitor.run(notifier, packet_sender).await;
}

fn new_packet_sender(
    directory_client: Arc<directory_client::Client>,
    good_topology: NymTopology,
    self_address: Recipient,
    gateway_client: GatewayClient,
    test_run_sender: TestRunUpdateSender,
) -> PacketSender {
    PacketSender::new(
        directory_client,
        good_topology,
        self_address,
        gateway_client,
        test_run_sender,
    )
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

fn new_directory_client(directory_uri: &str) -> Arc<directory_client::Client> {
    let config = directory_client::Config::new(directory_uri.to_string());
    Arc::new(DirectoryClient::new(config))
}

fn new_notifier(
    encryption_keypair: encryption::KeyPair,
    directory_client: Arc<directory_client::Client>,
    mixnet_receiver: MixnetReceiver,
    test_run_receiver: TestRunUpdateReceiver,
) -> Notifier {
    Notifier::new(
        mixnet_receiver,
        encryption_keypair,
        directory_client,
        test_run_receiver,
    )
}
