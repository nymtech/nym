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
use crate::tested_network::good_topology::parse_topology_file;
use crate::tested_network::TestedNetwork;
use clap::{App, Arg, ArgMatches};
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use gateway_client::GatewayClient;
use log::*;
use monitor::{AckSender, MixnetSender, Monitor};
use notifications::Notifier;
use nymsphinx::addressing::clients::Recipient;
use packet_sender::PacketSender;
use rand::rngs::OsRng;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use topology::{gateway, NymTopology};

mod chunker;
mod monitor;
mod notifications;
mod packet_sender;
mod run_info;
mod test_packet;
mod tested_network;

pub(crate) type DefRng = OsRng;
pub(crate) const DEFAULT_RNG: DefRng = OsRng;

const V4_TOPOLOGY_ARG: &str = "v4-topology-filepath";
const V6_TOPOLOGY_ARG: &str = "v6-topology-filepath";
const VALIDATOR_ARG: &str = "validator";
const DETAILED_REPORT_ARG: &str = "detailed-report";
const GATEWAY_SENDING_RATE_ARG: &str = "gateway-rate";

const DEFAULT_VALIDATOR: &str = "http://testnet-validator1.nymtech.net:8081";
const DEFAULT_GATEWAY_SENDING_RATE: usize = 500;
pub(crate) const TIME_CHUNK_SIZE: Duration = Duration::from_millis(200);

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Network Monitor")
        .author("Nymtech")
        .arg(
            Arg::with_name(V4_TOPOLOGY_ARG)
                .help("location of .json file containing IPv4 'good' network topology")
                .long(V4_TOPOLOGY_ARG)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(V6_TOPOLOGY_ARG)
                .help("location of .json file containing IPv6 'good' network topology")
                .long(V6_TOPOLOGY_ARG)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(VALIDATOR_ARG)
                .help("REST endpoint of the validator the monitor will grab nodes to test")
                .long(VALIDATOR_ARG)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DETAILED_REPORT_ARG)
                .help("specifies whether a detailed report should be printed after each run")
                .long(DETAILED_REPORT_ARG)
            ,
        )
        .arg(Arg::with_name(GATEWAY_SENDING_RATE_ARG)
            .help("specifies maximum rate (in packets per second) of test packets being sent to gateway")
            .takes_value(true)
            .long(GATEWAY_SENDING_RATE_ARG)
            .short("r")
        )
        .get_matches()
}

#[tokio::main]
async fn main() {
    println!("Network monitor starting...");
    dotenv::dotenv().ok();
    let matches = parse_args();
    let v4_topology_path = matches.value_of(V4_TOPOLOGY_ARG).unwrap();
    let v6_topology_path = matches.value_of(V6_TOPOLOGY_ARG).unwrap();

    let v4_topology = parse_topology_file(v4_topology_path);
    let v6_topology = parse_topology_file(v6_topology_path);

    let validator_rest_uri = matches
        .value_of(VALIDATOR_ARG)
        .unwrap_or_else(|| DEFAULT_VALIDATOR);
    let detailed_report = matches.is_present(DETAILED_REPORT_ARG);
    let sending_rate = matches
        .value_of(GATEWAY_SENDING_RATE_ARG)
        .map(|v| v.parse().unwrap())
        .unwrap_or_else(|| DEFAULT_GATEWAY_SENDING_RATE);

    check_if_up_to_date(&v4_topology, &v6_topology);
    setup_logging();

    println!("* validator server: {}", validator_rest_uri);

    // TODO: THIS MUST BE UPDATED!!
    // TODO: THIS MUST BE UPDATED!!
    // TODO: THIS MUST BE UPDATED!!
    warn!("using v4 gateway for both topologies!");
    let gateway = v4_topology.gateways()[0].clone();

    // TODO: this might change if it turns out we need both v4 and v6 gateway clients
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
        *identity_keypair.public_key(),
        *encryption_keypair.public_key(),
        gateway.identity_key,
    );

    let validator_client = new_validator_client(validator_rest_uri);

    let mut network_monitor = Monitor::new();

    let notifier = new_notifier(
        encryption_keypair,
        Arc::clone(&validator_client),
        mixnet_receiver,
        test_run_receiver,
        detailed_report,
    );

    let gateway_client = new_gateway_client(gateway, identity_keypair, ack_sender, mixnet_sender);
    let tested_network =
        new_tested_network(gateway_client, v4_topology, v6_topology, sending_rate).await;

    let packet_sender = new_packet_sender(
        validator_client,
        tested_network,
        self_address,
        test_run_sender,
    );

    network_monitor.run(notifier, packet_sender).await;
}

async fn new_tested_network(
    gateway_client: GatewayClient,
    good_v4_topology: NymTopology,
    good_v6_topology: NymTopology,
    max_sending_rate: usize,
) -> TestedNetwork {
    // TODO: possibly change that if it turns out we need two clients (v4 and v6)
    let mut tested_network = TestedNetwork::new_good(
        gateway_client,
        good_v4_topology,
        good_v6_topology,
        max_sending_rate,
    );
    tested_network.start_gateway_client().await;
    tested_network
}

fn new_packet_sender(
    validator_client: Arc<validator_client::Client>,
    tested_network: TestedNetwork,
    self_address: Recipient,
    test_run_sender: TestRunUpdateSender,
) -> PacketSender {
    PacketSender::new(
        validator_client,
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

fn new_validator_client(validator_rest_uri: &str) -> Arc<validator_client::Client> {
    let config = validator_client::Config::new(validator_rest_uri.to_string());
    Arc::new(validator_client::Client::new(config))
}

fn new_notifier(
    encryption_keypair: encryption::KeyPair,
    validator_client: Arc<validator_client::Client>,
    mixnet_receiver: MixnetReceiver,
    test_run_receiver: TestRunUpdateReceiver,
    with_detailed_report: bool,
) -> Notifier {
    Notifier::new(
        mixnet_receiver,
        encryption_keypair,
        validator_client,
        test_run_receiver,
        with_detailed_report,
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

fn check_if_up_to_date(v4_topology: &NymTopology, v6_topology: &NymTopology) {
    let monitor_version = env!("CARGO_PKG_VERSION");
    for (_, layer_mixes) in v4_topology.mixes().iter() {
        for mix in layer_mixes.iter() {
            if !version_checker::is_minor_version_compatible(monitor_version, &*mix.version) {
                panic!(
                    "Our good topology is not compatible with monitor! Mix runs {}, we have {}",
                    mix.version, monitor_version
                )
            }
        }
    }

    for gateway in v4_topology.gateways().iter() {
        if !version_checker::is_minor_version_compatible(monitor_version, &*gateway.version) {
            panic!(
                "Our good topology is not compatible with monitor! Gateway runs {}, we have {}",
                gateway.version, monitor_version
            )
        }
    }

    for (_, layer_mixes) in v6_topology.mixes().iter() {
        for mix in layer_mixes.iter() {
            if !version_checker::is_minor_version_compatible(monitor_version, &*mix.version) {
                panic!(
                    "Our good topology is not compatible with monitor! Mix runs {}, we have {}",
                    mix.version, monitor_version
                )
            }
        }
    }

    for gateway in v6_topology.gateways().iter() {
        if !version_checker::is_minor_version_compatible(monitor_version, &*gateway.version) {
            panic!(
                "Our good topology is not compatible with monitor! Gateway runs {}, we have {}",
                gateway.version, monitor_version
            )
        }
    }
}
