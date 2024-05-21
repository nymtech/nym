// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

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
use crate::nym_contract_cache::cache::NymContractCache;
use crate::storage::NymApiStorage;
use crate::support::{config, nyxd};
use futures::channel::mpsc;
use nym_bandwidth_controller::BandwidthController;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_crypto::asymmetric::{encryption, identity};
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::params::PacketType;
use nym_sphinx::receiver::MessageReceiver;
use nym_task::TaskManager;
use std::sync::Arc;

pub(crate) mod gateways_reader;
pub(crate) mod monitor;
pub(crate) mod test_packet;
pub(crate) mod test_route;

pub(crate) const ROUTE_TESTING_TEST_NONCE: u64 = 0;

pub(crate) fn setup<'a>(
    config: &'a config::NetworkMonitor,
    nym_contract_cache_state: &NymContractCache,
    storage: &NymApiStorage,
    nyxd_client: nyxd::Client,
) -> NetworkMonitorBuilder<'a> {
    NetworkMonitorBuilder::new(
        config,
        nyxd_client,
        storage.to_owned(),
        nym_contract_cache_state.to_owned(),
    )
}

pub(crate) struct NetworkMonitorBuilder<'a> {
    config: &'a config::NetworkMonitor,
    nyxd_client: nyxd::Client,
    node_status_storage: NymApiStorage,
    validator_cache: NymContractCache,
}

impl<'a> NetworkMonitorBuilder<'a> {
    pub(crate) fn new(
        config: &'a config::NetworkMonitor,
        nyxd_client: nyxd::Client,
        node_status_storage: NymApiStorage,
        validator_cache: NymContractCache,
    ) -> Self {
        NetworkMonitorBuilder {
            config,
            nyxd_client,
            node_status_storage,
            validator_cache,
        }
    }

    pub(crate) async fn build<R: MessageReceiver + Send + 'static>(
        self,
    ) -> NetworkMonitorRunnables<R> {
        // TODO: those keys change constant throughout the whole execution of the monitor.
        // and on top of that, they are used with ALL the gateways -> presumably this should change
        // in the future
        let mut rng = rand::rngs::OsRng;

        let identity_keypair = Arc::new(identity::KeyPair::new(&mut rng));
        let encryption_keypair = Arc::new(encryption::KeyPair::new(&mut rng));
        let ack_key = Arc::new(AckKey::new(&mut rng));

        let (gateway_status_update_sender, gateway_status_update_receiver) = mpsc::unbounded();
        let (received_processor_sender_channel, received_processor_receiver_channel) =
            mpsc::unbounded();

        let packet_preparer = new_packet_preparer(
            self.validator_cache,
            self.config.debug.per_node_test_packets,
            Arc::clone(&ack_key),
            *identity_keypair.public_key(),
            *encryption_keypair.public_key(),
        );

        let bandwidth_controller = {
            BandwidthController::new(
                nym_credential_storage::initialise_persistent_storage(
                    &self.config.storage_paths.credentials_database_path,
                )
                .await,
                self.nyxd_client.clone(),
            )
        };

        let packet_sender = new_packet_sender(
            self.config,
            gateway_status_update_sender,
            Arc::clone(&identity_keypair),
            self.config.debug.gateway_sending_rate,
            bandwidth_controller,
            self.config.debug.disabled_credentials_mode,
        );

        let received_processor = new_received_processor(
            received_processor_receiver_channel,
            Arc::clone(&encryption_keypair),
            ack_key,
        );
        let summary_producer = new_summary_producer(self.config.debug.per_node_test_packets);
        let packet_receiver = new_packet_receiver(
            gateway_status_update_receiver,
            received_processor_sender_channel,
        );

        let monitor = Monitor::new(
            self.config,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            self.node_status_storage,
            PacketType::Mix,
        );

        NetworkMonitorRunnables {
            monitor,
            packet_receiver,
        }
    }
}

pub(crate) struct NetworkMonitorRunnables<R: MessageReceiver + Send + 'static> {
    monitor: Monitor<R>,
    packet_receiver: PacketReceiver,
}

impl<R: MessageReceiver + Send + 'static> NetworkMonitorRunnables<R> {
    // TODO: note, that is not exactly doing what we want, because when
    // `ReceivedProcessor` is constructed, it already spawns a future
    // this needs to be refactored!
    pub(crate) fn spawn_tasks(self, shutdown: &TaskManager) {
        let mut packet_receiver = self.packet_receiver;
        let mut monitor = self.monitor;
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { packet_receiver.run(shutdown_listener).await });
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { monitor.run(shutdown_listener).await });
    }
}

fn new_packet_preparer(
    validator_cache: NymContractCache,
    per_node_test_packets: usize,
    ack_key: Arc<AckKey>,
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
) -> PacketPreparer {
    PacketPreparer::new(
        validator_cache,
        per_node_test_packets,
        ack_key,
        self_public_identity,
        self_public_encryption,
    )
}

fn new_packet_sender(
    config: &config::NetworkMonitor,
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<identity::KeyPair>,
    max_sending_rate: usize,
    bandwidth_controller: BandwidthController<nyxd::Client, PersistentStorage>,
    disabled_credentials_mode: bool,
) -> PacketSender {
    PacketSender::new(
        gateways_status_updater,
        local_identity,
        config.debug.gateway_response_timeout,
        config.debug.gateway_connection_timeout,
        config.debug.max_concurrent_gateway_clients,
        max_sending_rate,
        bandwidth_controller,
        disabled_credentials_mode,
    )
}

fn new_received_processor<R: MessageReceiver + Send + 'static>(
    packets_receiver: ReceivedProcessorReceiver,
    client_encryption_keypair: Arc<encryption::KeyPair>,
    ack_key: Arc<AckKey>,
) -> ReceivedProcessor<R> {
    ReceivedProcessor::new(packets_receiver, client_encryption_keypair, ack_key)
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

// TODO: 1) does it still have to have separate builder or could we get rid of it now?
// TODO: 2) how do we make it non-async as other 'start' methods?
pub(crate) async fn start<R: MessageReceiver + Send + 'static>(
    config: &config::NetworkMonitor,
    nym_contract_cache_state: &NymContractCache,
    storage: &NymApiStorage,
    nyxd_client: nyxd::Client,
    shutdown: &TaskManager,
) {
    let monitor_builder = setup(config, nym_contract_cache_state, storage, nyxd_client);
    info!("Starting network monitor...");
    let runnables: NetworkMonitorRunnables<R> = monitor_builder.build().await;
    runnables.spawn_tasks(shutdown);
}
