// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::ValidatorCache;
use crate::config::Config;
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
use crate::storage::ValidatorApiStorage;
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use log::info;
use nymsphinx::addressing::clients::Recipient;
use std::sync::Arc;
use topology::NymTopology;

#[cfg(feature = "coconut")]
use coconut_interface::Credential;
#[cfg(feature = "coconut")]
use credentials::{bandwidth::prepare_for_spending, obtain_aggregate_verification_key};

pub(crate) mod chunker;
pub(crate) mod gateways_reader;
pub(crate) mod monitor;
pub(crate) mod test_packet;
pub(crate) mod tested_network;

pub(crate) struct NetworkMonitorBuilder<'a> {
    config: &'a Config,
    tested_network: TestedNetwork,
    node_status_storage: ValidatorApiStorage,
    validator_cache: ValidatorCache,
}

impl<'a> NetworkMonitorBuilder<'a> {
    pub(crate) fn new(
        config: &'a Config,
        v4_topology: NymTopology,
        v6_topology: NymTopology,
        node_status_storage: ValidatorApiStorage,
        validator_cache: ValidatorCache,
    ) -> Self {
        let tested_network = TestedNetwork::new_good(v4_topology, v6_topology);

        NetworkMonitorBuilder {
            config,
            tested_network,
            node_status_storage,
            validator_cache,
        }
    }

    pub(crate) async fn build(self) -> NetworkMonitorRunnables {
        // TODO: in the future I guess this should somehow change to distribute the load
        let tested_mix_gateway = self.tested_network.main_v4_gateway().clone();
        info!(
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

        let (gateway_status_update_sender, gateway_status_update_receiver) = mpsc::unbounded();
        let (received_processor_sender_channel, received_processor_receiver_channel) =
            mpsc::unbounded();

        let packet_preparer = new_packet_preparer(
            self.validator_cache,
            self.tested_network.clone(),
            test_mixnode_sender,
            *identity_keypair.public_key(),
            *encryption_keypair.public_key(),
        );

        #[cfg(feature = "coconut")]
        let bandwidth_credential =
            TEMPORARY_obtain_bandwidth_credential(self.config, identity_keypair.public_key()).await;

        let packet_sender = new_packet_sender(
            self.config,
            gateway_status_update_sender,
            Arc::clone(&identity_keypair),
            self.config.get_gateway_sending_rate(),
            #[cfg(feature = "coconut")]
            bandwidth_credential,
        );

        let received_processor = new_received_processor(
            received_processor_receiver_channel,
            Arc::clone(&encryption_keypair),
        );
        let summary_producer = new_summary_producer(self.config.get_detailed_report());
        let packet_receiver = new_packet_receiver(
            gateway_status_update_receiver,
            received_processor_sender_channel,
        );

        let monitor = monitor::Monitor::new(
            self.config,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            self.node_status_storage,
            self.tested_network,
        );

        NetworkMonitorRunnables {
            monitor,
            packet_receiver,
        }
    }
}

pub(crate) struct NetworkMonitorRunnables {
    monitor: Monitor,
    packet_receiver: PacketReceiver,
}

impl NetworkMonitorRunnables {
    // TODO: note, that is not exactly doing what we want, because when
    // `ReceivedProcessor` is constructed, it already spawns a future
    // this needs to be refactored!
    pub(crate) fn spawn_tasks(self) {
        let mut packet_receiver = self.packet_receiver;
        let mut monitor = self.monitor;
        tokio::spawn(async move { packet_receiver.run().await });
        tokio::spawn(async move { monitor.run().await });
    }
}

fn new_packet_preparer(
    validator_cache: ValidatorCache,
    tested_network: TestedNetwork,
    test_mixnode_sender: Recipient,
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
) -> PacketPreparer {
    PacketPreparer::new(
        validator_cache,
        tested_network,
        test_mixnode_sender,
        self_public_identity,
        self_public_encryption,
    )
}

// SECURITY:
// this implies we are re-using the same credential for all gateways all the time (which unfortunately is true!)
#[cfg(feature = "coconut")]
#[allow(non_snake_case)]
async fn TEMPORARY_obtain_bandwidth_credential(
    config: &Config,
    identity: &identity::PublicKey,
) -> Credential {
    info!("Trying to obtain bandwidth credential...");
    let validators = config.get_all_validator_api_endpoints();

    let verification_key = obtain_aggregate_verification_key(&validators)
        .await
        .expect("could not obtain aggregate verification key of ALL validators");

    let bandwidth_credential =
        credentials::bandwidth::obtain_signature(&identity.to_bytes(), &validators)
            .await
            .expect("failed to obtain bandwidth credential!");

    prepare_for_spending(
        &identity.to_bytes(),
        &bandwidth_credential,
        &verification_key,
    )
    .expect("failed to prepare bandwidth credential for spending!")
}

fn new_packet_sender(
    config: &Config,
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<identity::KeyPair>,
    max_sending_rate: usize,
    #[cfg(feature = "coconut")] bandwidth_credential: Credential,
) -> PacketSender {
    PacketSender::new(
        gateways_status_updater,
        local_identity,
        config.get_gateway_response_timeout(),
        config.get_gateway_connection_timeout(),
        config.get_max_concurrent_gateway_clients(),
        max_sending_rate,
        #[cfg(feature = "coconut")]
        bandwidth_credential,
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
