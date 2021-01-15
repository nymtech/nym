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

use crate::monitor::preparer::PacketPreparer;
use crate::monitor::processor::{
    ReceivedProcessor, ReceivedProcessorReceiver, ReceivedProcessorSender,
};
use crate::monitor::receiver::{
    GatewayClientUpdateReceiver, GatewayClientUpdateSender, PacketReceiver,
};
use crate::monitor::sender::PacketSender;
use crate::monitor::summary_producer::SummaryProducer;
use crate::run_info::{TestRunUpdateReceiver, TestRunUpdateSender};
use crate::tested_network::good_topology::parse_topology_file;
use crate::tested_network::TestedNetwork;
use clap::{App, Arg, ArgMatches};
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use gateway_client::{
    AcknowledgementSender, GatewayClient, MixnetMessageReceiver, MixnetMessageSender,
};
use log::*;
use monitor_old::Monitor;
use notifications::Notifier;
use nymsphinx::addressing::clients::Recipient;
use packet_sender::PacketSenderOld;
use rand::rngs::OsRng;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use topology::{gateway, NymTopology};

mod chunker;
pub(crate) mod gateways_reader;
mod mixnet_receiver;
mod monitor;
mod monitor_old;
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
pub(crate) const TIME_CHUNK_SIZE: Duration = Duration::from_millis(50);

pub(crate) const PENALISE_OUTDATED: bool = false;

// TODO: let's see how it goes and whether those new adjusting
const MAX_CONCURRENT_GATEWAY_CLIENTS: Option<usize> = Some(50);
const GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);

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

    // TODO: in the future I guess this should somehow change to distribute the load
    let tested_mix_gateway = v4_topology.gateways()[0].clone();
    println!(
        "* gateway for testing mixnodes: {}",
        tested_mix_gateway.identity_key.to_base58_string()
    );

    // TODO: those keys change constant throughout the whole execution of the monitor.
    // and on top of that, they are used with ALL the gateways -> presumably this should change
    // in the future
    let identity_keypair = Arc::new(identity::KeyPair::new());
    let encryption_keypair = Arc::new(encryption::KeyPair::new());

    let test_mixnode_sender = Recipient::new(
        *identity_keypair.public_key(),
        *encryption_keypair.public_key(),
        tested_mix_gateway.identity_key,
    );

    let tested_network = TestedNetwork::new_good(v4_topology, v6_topology);
    let validator_client = new_validator_client(validator_rest_uri);

    let (gateway_status_update_sender, gateway_status_update_receiver) = mpsc::unbounded();
    let (received_processor_sender_channel, received_processor_receiver_channel) =
        mpsc::unbounded();

    let packet_preparer = new_packet_preparer(
        Arc::clone(&validator_client),
        tested_network,
        test_mixnode_sender,
        *identity_keypair.public_key(),
        *encryption_keypair.public_key(),
    );
    let packet_sender =
        new_packet_sender(gateway_status_update_sender, Arc::clone(&identity_keypair));
    let received_processor = new_received_processor(
        received_processor_receiver_channel,
        Arc::clone(&encryption_keypair),
    );
    let summary_producer = new_summary_producer(detailed_report);
    let mut packet_receiver = new_packet_receiver(
        gateway_status_update_receiver,
        received_processor_sender_channel,
    );

    let mut monitor = monitor::Monitor::new(
        packet_preparer,
        packet_sender,
        received_processor,
        summary_producer,
        validator_client,
    );

    tokio::spawn(async move { packet_receiver.run().await });

    tokio::spawn(async move { monitor.run().await });

    wait_for_interrupt().await
}

async fn wait_for_interrupt() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(
            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
            e
        );
    }
    println!("Received SIGINT - the network monitor will terminate now");
}

fn new_packet_preparer(
    validator_client: Arc<validator_client::Client>,
    tested_network: TestedNetwork,
    test_mixnode_sender: Recipient,
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
) -> PacketPreparer {
    PacketPreparer::new(
        validator_client,
        tested_network,
        test_mixnode_sender,
        self_public_identity,
        self_public_encryption,
    )
}

fn new_packet_sender(
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<identity::KeyPair>,
) -> PacketSender {
    PacketSender::new(
        gateways_status_updater,
        local_identity,
        GATEWAY_RESPONSE_TIMEOUT,
        MAX_CONCURRENT_GATEWAY_CLIENTS,
    )
}

fn new_received_processor(
    packets_receiver: ReceivedProcessorReceiver,
    client_encryption_keypair: Arc<encryption::KeyPair>,
) -> ReceivedProcessor {
    ReceivedProcessor::new(packets_receiver, client_encryption_keypair)
}

fn new_summary_producer(detailed_report: bool) -> SummaryProducer {
    // right now always print the basic report. If we feel like we need to change it, it can
    // be easily adjusted by adding some flag or something
    let mut summary_producer = SummaryProducer::default().with_report();
    if detailed_report {
        summary_producer.with_detailed_report()
    } else {
        summary_producer
    }
}

fn new_packet_receiver(
    gateways_status_updater: GatewayClientUpdateReceiver,
    processor_packets_sender: ReceivedProcessorSender,
) -> PacketReceiver {
    PacketReceiver::new(gateways_status_updater, processor_packets_sender)
}

async fn main_old() {
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
    let tested_network = todo!();
    // new_tested_network(gateway_client, v4_topology, v6_topology, sending_rate).await;

    let packet_sender = new_packet_sender_old(
        validator_client,
        tested_network,
        self_address,
        test_run_sender,
    );

    network_monitor.run(notifier, packet_sender).await;
}

fn new_packet_sender_old(
    validator_client: Arc<validator_client::Client>,
    tested_network: TestedNetwork,
    self_address: Recipient,
    test_run_sender: TestRunUpdateSender,
) -> PacketSenderOld {
    PacketSenderOld::new(
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
    ack_sender: AcknowledgementSender,
    mixnet_messages_sender: MixnetMessageSender,
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
    mixnet_receiver: MixnetMessageReceiver,
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
