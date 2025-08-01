// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
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
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_status_api::NodeStatusCache;
use crate::storage::NymApiStorage;
use crate::support::caching::cache::SharedCache;
use crate::support::config::Config;
use crate::support::nyxd;
use futures::channel::mpsc;
use nym_bandwidth_controller::BandwidthController;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::params::PacketType;
use nym_sphinx::receiver::MessageReceiver;
use nym_task::TaskManager;
use std::sync::Arc;
use tracing::info;

pub(crate) mod gateways_reader;
pub(crate) mod monitor;
pub(crate) mod test_packet;
pub(crate) mod test_route;

pub(crate) const ROUTE_TESTING_TEST_NONCE: u64 = 0;

pub(crate) fn setup<'a>(
    config: &'a Config,
    nym_contract_cache: &MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,
    node_status_cache: NodeStatusCache,
    storage: &NymApiStorage,
    nyxd_client: nyxd::Client,
) -> NetworkMonitorBuilder<'a> {
    NetworkMonitorBuilder::new(
        config,
        nyxd_client,
        storage.to_owned(),
        nym_contract_cache.clone(),
        described_cache,
        node_status_cache,
    )
}

pub(crate) struct NetworkMonitorBuilder<'a> {
    config: &'a Config,
    nyxd_client: nyxd::Client,
    node_status_storage: NymApiStorage,
    contract_cache: MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,
    node_status_cache: NodeStatusCache,
}

impl<'a> NetworkMonitorBuilder<'a> {
    pub(crate) fn new(
        config: &'a Config,
        nyxd_client: nyxd::Client,
        node_status_storage: NymApiStorage,
        contract_cache: MixnetContractCache,
        described_cache: SharedCache<DescribedNodes>,
        node_status_cache: NodeStatusCache,
    ) -> Self {
        NetworkMonitorBuilder {
            config,
            nyxd_client,
            node_status_storage,
            contract_cache,
            described_cache,
            node_status_cache,
        }
    }

    pub(crate) async fn build<R: MessageReceiver + Send + Sync + 'static>(
        self,
    ) -> NetworkMonitorRunnables<R> {
        // TODO: those keys change constant throughout the whole execution of the monitor.
        // and on top of that, they are used with ALL the gateways -> presumably this should change
        // in the future
        let mut rng = rand::rngs::OsRng;

        let identity_keypair = Arc::new(ed25519::KeyPair::new(&mut rng));
        let encryption_keypair = Arc::new(x25519::KeyPair::new(&mut rng));
        let ack_key = Arc::new(AckKey::new(&mut rng));

        let (gateway_status_update_sender, gateway_status_update_receiver) = mpsc::unbounded();
        let (received_processor_sender_channel, received_processor_receiver_channel) =
            mpsc::unbounded();

        let packet_preparer = new_packet_preparer(
            self.contract_cache,
            self.described_cache,
            self.node_status_cache,
            self.config.network_monitor.debug.per_node_test_packets,
            Arc::clone(&ack_key),
            *identity_keypair.public_key(),
            *encryption_keypair.public_key(),
        );

        let bandwidth_controller = {
            BandwidthController::new(
                nym_credential_storage::initialise_persistent_storage(
                    &self
                        .config
                        .network_monitor
                        .storage_paths
                        .credentials_database_path,
                )
                .await,
                self.nyxd_client.clone(),
            )
        };

        let packet_sender = new_packet_sender(
            self.config,
            gateway_status_update_sender,
            Arc::clone(&identity_keypair),
            bandwidth_controller,
        );

        let received_processor = new_received_processor(
            received_processor_receiver_channel,
            Arc::clone(&encryption_keypair),
            ack_key,
        );
        let summary_producer =
            new_summary_producer(self.config.network_monitor.debug.per_node_test_packets);
        let packet_receiver = new_packet_receiver(
            gateway_status_update_receiver,
            received_processor_sender_channel,
        );

        let monitor = Monitor::new(
            &self.config.network_monitor,
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

pub(crate) struct NetworkMonitorRunnables<R: MessageReceiver + Send + Sync + 'static> {
    monitor: Monitor<R>,
    packet_receiver: PacketReceiver,
}

impl<R: MessageReceiver + Send + Sync + 'static> NetworkMonitorRunnables<R> {
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
    contract_cache: MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,
    node_status_cache: NodeStatusCache,
    per_node_test_packets: usize,
    ack_key: Arc<AckKey>,
    self_public_identity: ed25519::PublicKey,
    self_public_encryption: x25519::PublicKey,
) -> PacketPreparer {
    PacketPreparer::new(
        contract_cache,
        described_cache,
        node_status_cache,
        per_node_test_packets,
        ack_key,
        self_public_identity,
        self_public_encryption,
    )
}

fn new_packet_sender(
    config: &Config,
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<ed25519::KeyPair>,
    bandwidth_controller: BandwidthController<nyxd::Client, PersistentStorage>,
) -> PacketSender {
    PacketSender::new(
        config,
        gateways_status_updater,
        local_identity,
        bandwidth_controller,
    )
}

fn new_received_processor<R: MessageReceiver + Send + 'static>(
    packets_receiver: ReceivedProcessorReceiver,
    client_encryption_keypair: Arc<x25519::KeyPair>,
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
pub(crate) async fn start<R: MessageReceiver + Send + Sync + 'static>(
    config: &Config,
    nym_contract_cache: &MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,
    node_status_cache: NodeStatusCache,
    storage: &NymApiStorage,
    nyxd_client: nyxd::Client,
    shutdown: &TaskManager,
) {
    let monitor_builder = setup(
        config,
        nym_contract_cache,
        described_cache,
        node_status_cache,
        storage,
        nyxd_client,
    );
    info!("Starting network monitor...");
    let runnables: NetworkMonitorRunnables<R> = monitor_builder.build().await;
    runnables.spawn_tasks(shutdown);
}
