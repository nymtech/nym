// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use credential_storage::PersistentStorage;
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use gateway_client::bandwidth::BandwidthController;
use std::sync::Arc;

use crate::config::Config;
use crate::contract_cache::ValidatorCache;
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
use crate::storage::ValidatorApiStorage;

pub(crate) mod chunker;
pub(crate) mod gateways_reader;
pub(crate) mod monitor;
pub(crate) mod test_packet;
pub(crate) mod test_route;

pub(crate) const ROUTE_TESTING_TEST_NONCE: u64 = 0;

pub(crate) struct NetworkMonitorBuilder<'a> {
    config: &'a Config,
    system_version: String,
    node_status_storage: ValidatorApiStorage,
    validator_cache: ValidatorCache,
}

impl<'a> NetworkMonitorBuilder<'a> {
    pub(crate) fn new(
        config: &'a Config,
        system_version: &str,
        node_status_storage: ValidatorApiStorage,
        validator_cache: ValidatorCache,
    ) -> Self {
        NetworkMonitorBuilder {
            config,
            system_version: system_version.to_string(),
            node_status_storage,
            validator_cache,
        }
    }

    pub(crate) async fn build(self) -> NetworkMonitorRunnables {
        // TODO: those keys change constant throughout the whole execution of the monitor.
        // and on top of that, they are used with ALL the gateways -> presumably this should change
        // in the future
        let mut rng = rand_07::rngs::OsRng;

        let identity_keypair = Arc::new(identity::KeyPair::new(&mut rng));
        let encryption_keypair = Arc::new(encryption::KeyPair::new(&mut rng));

        let (gateway_status_update_sender, gateway_status_update_receiver) = mpsc::unbounded();
        let (received_processor_sender_channel, received_processor_receiver_channel) =
            mpsc::unbounded();

        let packet_preparer = new_packet_preparer(
            &self.system_version,
            self.validator_cache,
            self.config.get_per_node_test_packets(),
            *identity_keypair.public_key(),
            *encryption_keypair.public_key(),
        );

        #[cfg(feature = "coconut")]
        let bandwidth_controller = BandwidthController::new(
            credential_storage::initialise_storage(self.config.get_credentials_database_path())
                .await,
            self.config.get_all_validator_api_endpoints(),
        );
        #[cfg(not(feature = "coconut"))]
        let bandwidth_controller = BandwidthController::new(
            credential_storage::initialise_storage(self.config.get_credentials_database_path())
                .await,
            self.config.get_network_monitor_eth_endpoint(),
            self.config.get_network_monitor_eth_private_key(),
        )
        .expect("Could not create bandwidth controller");

        let packet_sender = new_packet_sender(
            self.config,
            gateway_status_update_sender,
            Arc::clone(&identity_keypair),
            self.config.get_gateway_sending_rate(),
            bandwidth_controller,
            self.config.get_testnet_mode(),
        );

        let received_processor = new_received_processor(
            received_processor_receiver_channel,
            Arc::clone(&encryption_keypair),
        );
        let summary_producer = new_summary_producer(self.config.get_per_node_test_packets());
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
    system_version: &str,
    validator_cache: ValidatorCache,
    per_node_test_packets: usize,
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
) -> PacketPreparer {
    PacketPreparer::new(
        system_version,
        validator_cache,
        per_node_test_packets,
        self_public_identity,
        self_public_encryption,
    )
}

fn new_packet_sender(
    config: &Config,
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<identity::KeyPair>,
    max_sending_rate: usize,
    bandwidth_controller: BandwidthController<PersistentStorage>,
    testnet_mode: bool,
) -> PacketSender {
    PacketSender::new(
        gateways_status_updater,
        local_identity,
        config.get_gateway_response_timeout(),
        config.get_gateway_connection_timeout(),
        config.get_max_concurrent_gateway_clients(),
        max_sending_rate,
        bandwidth_controller,
        testnet_mode,
    )
}

fn new_received_processor(
    packets_receiver: ReceivedProcessorReceiver,
    client_encryption_keypair: Arc<encryption::KeyPair>,
) -> ReceivedProcessor {
    ReceivedProcessor::new(packets_receiver, client_encryption_keypair)
}

fn new_summary_producer(per_node_test_packets: usize) -> SummaryProducer {
    // right now always print the basic report. If we feel like we need to change it, it can
    // be easily adjusted by adding some flag or something
    SummaryProducer::new(per_node_test_packets).with_report()
}

fn new_packet_receiver(
    gateways_status_updater: GatewayClientUpdateReceiver,
    processor_packets_sender: ReceivedProcessorSender,
) -> PacketReceiver {
    PacketReceiver::new(gateways_status_updater, processor_packets_sender)
}
