// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::preparer::PacketPreparer;
use crate::network_monitor::monitor::processor::{
    ReceivedProcessor, ReceivedProcessorReceiver, ReceivedProcessorSender,
};
use crate::network_monitor::monitor::receiver::{
    GatewayClientUpdateReceiver, GatewayClientUpdateSender, PacketReceiver,
};
use crate::network_monitor::monitor::sender::PacketSender;
use crate::network_monitor::monitor::summary_producer::SummaryProducer;
use crate::network_monitor::monitor::Monitor;
use crate::network_monitor::tested_network::TestedNetwork;
use crate::{node_status_api, GATEWAY_RESPONSE_TIMEOUT, MAX_CONCURRENT_GATEWAY_CLIENTS};
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;
use std::sync::Arc;
use topology::NymTopology;

pub(crate) mod chunker;
pub(crate) mod gateways_reader;
pub(crate) mod monitor;
pub(crate) mod test_packet;
pub(crate) mod tested_network;

pub(crate) struct NetworkMonitorRunnables {
    monitor: Monitor,
    packet_receiver: PacketReceiver,
}

impl NetworkMonitorRunnables {
    pub(crate) fn spawn_tasks(self) {
        let mut packet_receiver = self.packet_receiver;
        let mut monitor = self.monitor;
        tokio::spawn(async move { packet_receiver.run().await });
        tokio::spawn(async move { monitor.run().await });
    }
}

pub(crate) fn new_monitor_runnables(
    validators_rest_uris: Vec<String>,
    mixnet_contract: &str,
    node_status_api_uri: &str,
    v4_topology: NymTopology,
    v6_topology: NymTopology,

    // those will be replaced by a config
    sending_rate: usize,
    detailed_report: bool,
) -> NetworkMonitorRunnables {
    // TODO: in the future I guess this should somehow change to distribute the load
    let tested_mix_gateway = v4_topology.gateways()[0].clone();
    info!(
        "* gateway for testing mixnodes: {}",
        tested_mix_gateway.identity_key.to_base58_string()
    );

    let tested_network = TestedNetwork::new_good(v4_topology, v6_topology);

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

    let monitor = monitor::Monitor::new(
        packet_preparer,
        packet_sender,
        received_processor,
        summary_producer,
        node_status_api_client,
        tested_network,
    );

    NetworkMonitorRunnables {
        monitor,
        packet_receiver,
    }
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

pub(crate) fn check_if_up_to_date(v4_topology: &NymTopology, v6_topology: &NymTopology) {
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

fn new_node_status_api_client<S: Into<String>>(base_url: S) -> node_status_api::Client {
    let config = node_status_api::Config::new(base_url);
    node_status_api::Client::new(config)
}

fn new_validator_client(
    validator_rest_uris: Vec<String>,
    mixnet_contract: &str,
) -> validator_client::Client {
    let config = validator_client::Config::new(validator_rest_uris, mixnet_contract);
    validator_client::Client::new(config)
}
