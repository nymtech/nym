// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::monitor::preparer::PacketPreparer;
use crate::monitor::processor::{
    ReceivedProcessor, ReceivedProcessorReceiver, ReceivedProcessorSender,
};
use crate::monitor::receiver::{
    GatewayClientUpdateReceiver, GatewayClientUpdateSender, PacketReceiver,
};
use crate::monitor::sender::PacketSender;
use crate::monitor::summary_producer::SummaryProducer;
use crate::tested_network::good_topology::parse_topology_file;
use crate::tested_network::TestedNetwork;
use clap::{App, Arg, ArgMatches};
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use std::sync::Arc;
use std::time::Duration;
use topology::NymTopology;

mod chunker;
pub(crate) mod gateways_reader;
mod monitor;
mod node_status_api;
mod test_packet;
mod tested_network;

const V4_TOPOLOGY_ARG: &str = "v4-topology-filepath";
const V6_TOPOLOGY_ARG: &str = "v6-topology-filepath";
const VALIDATORS_ARG: &str = "validators";
const NODE_STATUS_API_ARG: &str = "node-status-api";
const DETAILED_REPORT_ARG: &str = "detailed-report";
const GATEWAY_SENDING_RATE_ARG: &str = "gateway-rate";
const MIXNET_CONTRACT_ARG: &str = "mixnet-contract";

const DEFAULT_VALIDATORS: &[&str] = &[
    // "http://testnet-finney-validator.nymtech.net:1317",
    "http://testnet-finney-validator2.nymtech.net:1317",
    "http://mixnet.club:1317",
];

const DEFAULT_NODE_STATUS_API: &str = "http://localhost:8081";
const DEFAULT_GATEWAY_SENDING_RATE: usize = 500;
const DEFAULT_MIXNET_CONTRACT: &str = "hal1k0jntykt7e4g3y88ltc60czgjuqdy4c9c6gv94";

pub(crate) const TIME_CHUNK_SIZE: Duration = Duration::from_millis(50);
pub(crate) const PENALISE_OUTDATED: bool = false;

// TODO: let's see how it goes and whether those new adjusting
const MAX_CONCURRENT_GATEWAY_CLIENTS: Option<usize> = Some(50);
const GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);
pub(crate) const GATEWAY_CONNECTION_TIMEOUT: Duration = Duration::from_millis(2_500);

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
            Arg::with_name(VALIDATORS_ARG)
                .help("REST endpoint of the validator the monitor will grab nodes to test")
                .long(VALIDATORS_ARG)
                .takes_value(true)
        )
        .arg(Arg::with_name("mixnet-contract")
                 .long(MIXNET_CONTRACT_ARG)
                 .help("Address of the validator contract managing the network")
                 .takes_value(true),
        )
        .arg(
            Arg::with_name(NODE_STATUS_API_ARG)
                .help("Address of the node status api to submit results to. Most likely it's a local address")
                .long(NODE_STATUS_API_ARG)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DETAILED_REPORT_ARG)
                .help("specifies whether a detailed report should be printed after each run")
                .long(DETAILED_REPORT_ARG)
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

    let validators_rest_uris_borrowed = matches
        .values_of(VALIDATORS_ARG)
        .map(|args| args.collect::<Vec<_>>())
        .unwrap_or_else(|| DEFAULT_VALIDATORS.to_vec());

    let validators_rest_uris = validators_rest_uris_borrowed
        .into_iter()
        .map(|uri| uri.to_string())
        .collect::<Vec<_>>();

    let node_status_api_uri = matches
        .value_of(NODE_STATUS_API_ARG)
        .unwrap_or(DEFAULT_NODE_STATUS_API);

    let mixnet_contract = matches
        .value_of(MIXNET_CONTRACT_ARG)
        .unwrap_or(DEFAULT_MIXNET_CONTRACT);

    let detailed_report = matches.is_present(DETAILED_REPORT_ARG);
    let sending_rate = matches
        .value_of(GATEWAY_SENDING_RATE_ARG)
        .map(|v| v.parse().unwrap())
        .unwrap_or_else(|| DEFAULT_GATEWAY_SENDING_RATE);

    check_if_up_to_date(&v4_topology, &v6_topology);
    setup_logging();

    println!("* validator servers: {:?}", validators_rest_uris);
    println!("* node status api server: {}", node_status_api_uri);
    println!("* mixnet contract: {}", mixnet_contract);
    println!("* detailed report printing: {}", detailed_report);
    println!("* gateway sending rate: {} packets/s", sending_rate);

    // TODO: in the future I guess this should somehow change to distribute the load
    let tested_mix_gateway = v4_topology.gateways()[0].clone();
    println!(
        "* gateway for testing mixnodes: {}",
        tested_mix_gateway.identity_key.to_base58_string()
    );

    // TODO: those keys change constant throughout the whole execution of the monitor.
    // and on top of that, they are used with ALL the gateways -> presumably this should change
    // in the future
    let mut rng = rand::rngs::OsRng;

    let identity_keypair = Arc::new(identity::KeyPair::new(&mut rng));
    let encryption_keypair = Arc::new(encryption::KeyPair::new(&mut rng));

    let test_mixnode_sender = Recipient::new(
        *identity_keypair.public_key(),
        *encryption_keypair.public_key(),
        tested_mix_gateway.identity_key,
    );

    let tested_network = TestedNetwork::new_good(v4_topology, v6_topology);
    let validator_client = new_validator_client(validators_rest_uris, mixnet_contract);
    let node_status_api_client = new_node_status_api_client(node_status_api_uri);

    let (gateway_status_update_sender, gateway_status_update_receiver) = mpsc::unbounded();
    let (received_processor_sender_channel, received_processor_receiver_channel) =
        mpsc::unbounded();

    let packet_preparer = new_packet_preparer(
        validator_client,
        tested_network.clone(),
        test_mixnode_sender,
        *identity_keypair.public_key(),
        *encryption_keypair.public_key(),
    );

    let packet_sender = new_packet_sender(
        gateway_status_update_sender,
        Arc::clone(&identity_keypair),
        sending_rate,
    );
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
        node_status_api_client,
        tested_network,
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
    validator_client: validator_client::Client,
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
    max_sending_rate: usize,
) -> PacketSender {
    PacketSender::new(
        gateways_status_updater,
        local_identity,
        GATEWAY_RESPONSE_TIMEOUT,
        MAX_CONCURRENT_GATEWAY_CLIENTS,
        max_sending_rate,
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
    let summary_producer = SummaryProducer::default().with_report();
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

fn new_validator_client(
    validator_rest_uris: Vec<String>,
    mixnet_contract: &str,
) -> validator_client::Client {
    let config = validator_client::Config::new(validator_rest_uris, mixnet_contract);
    validator_client::Client::new(config)
}

fn new_node_status_api_client<S: Into<String>>(base_url: S) -> node_status_api::Client {
    let config = node_status_api::Config::new(base_url);
    node_status_api::Client::new(config)
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
