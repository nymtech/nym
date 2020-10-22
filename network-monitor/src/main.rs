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
use crate::run_info::{TestRunUpdateReceiver, TestRunUpdateSender};
use crate::tested_network::{good_topology, TestedNetwork};
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
use topology::gateway;

mod chunker;
mod monitor;
mod notifications;
mod packet_sender;
mod run_info;
mod test_packet;
mod tested_network;

pub(crate) type DefRng = OsRng;
pub(crate) const DEFAULT_RNG: DefRng = OsRng;

// CHANGE THIS TO GET COMPLETE LIST OF WHICH NODE IS WORKING OR BROKEN IN PARTICULAR WAY
// ||
// \/
pub const PRINT_DETAILED_REPORT: bool = false;
// /\
// ||
// CHANGE THIS TO GET COMPLETE LIST OF WHICH NODE IS WORKING OR BROKEN IN PARTICULAR WAY

#[tokio::main]
async fn main() {
    println!("Network monitor starting...");
    check_if_up_to_date();
    setup_logging();

    // Set up topology
    let directory_uri = "https://qa-directory.nymtech.net";
    println!("* directory server: {}", directory_uri);

    // TODO: this might change if it turns out we need both v4 and v6 gateway clients
    let gateway = tested_network::v4_gateway();
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
    let tested_network = new_tested_network(gateway_client).await;

    let packet_sender = new_packet_sender(
        directory_client,
        tested_network,
        self_address,
        test_run_sender,
    );

    network_monitor.run(notifier, packet_sender).await;
}

async fn new_tested_network(gateway_client: GatewayClient) -> TestedNetwork {
    // TODO: possibly change that if it turns out we need two clients (v4 and v6)
    let mut tested_network = TestedNetwork::new_good(gateway_client);
    tested_network.start_gateway_client().await;
    tested_network
}

fn new_packet_sender(
    directory_client: Arc<directory_client::Client>,
    tested_network: TestedNetwork,
    self_address: Recipient,
    test_run_sender: TestRunUpdateSender,
) -> PacketSender {
    PacketSender::new(
        directory_client,
        tested_network,
        self_address,
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

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .init();
}

fn check_if_up_to_date() {
    let monitor_version = env!("CARGO_PKG_VERSION");
    let good_v4_topology = good_topology::new_v4();
    for (_, layer_mixes) in good_v4_topology.mixes().into_iter() {
        for mix in layer_mixes.into_iter() {
            if !version_checker::is_minor_version_compatible(monitor_version, &*mix.version) {
                panic!(
                    "Our good topology is not compatible with monitor! Mix runs {}, we have {}",
                    mix.version, monitor_version
                )
            }
        }
    }

    for gateway in good_v4_topology.gateways().into_iter() {
        if !version_checker::is_minor_version_compatible(monitor_version, &*gateway.version) {
            panic!(
                "Our good topology is not compatible with monitor! Gateway runs {}, we have {}",
                gateway.version, monitor_version
            )
        }
    }

    let good_v6_topology = good_topology::new_v6();
    for (_, layer_mixes) in good_v6_topology.mixes().into_iter() {
        for mix in layer_mixes.into_iter() {
            if !version_checker::is_minor_version_compatible(monitor_version, &*mix.version) {
                panic!(
                    "Our good topology is not compatible with monitor! Mix runs {}, we have {}",
                    mix.version, monitor_version
                )
            }
        }
    }

    for gateway in good_v6_topology.gateways().into_iter() {
        if !version_checker::is_minor_version_compatible(monitor_version, &*gateway.version) {
            panic!(
                "Our good topology is not compatible with monitor! Gateway runs {}, we have {}",
                gateway.version, monitor_version
            )
        }
    }
}
