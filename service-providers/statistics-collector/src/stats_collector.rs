// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use nym_client_core::{HardcodedTopologyProvider, TopologyProvider};
use nym_sdk::GatewayTransceiver;
use nym_task::{TaskClient, TaskHandle};

use crate::{config::Config, error::StatsCollectorError, storage::ClientStatsStorage};

pub struct StatisticsCollector {
    #[allow(unused)]
    config: Config,
    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    shutdown: Option<TaskClient>,
}

impl StatisticsCollector {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
            shutdown: None,
        }
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_shutdown(mut self, shutdown: TaskClient) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_report_database_path<P: Into<std::path::PathBuf>>(
        mut self,
        database_path: P,
    ) -> Self {
        self.config.storage_paths.client_reports_database = database_path.into();
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_wait_for_gateway(mut self, wait_for_gateway: bool) -> Self {
        self.wait_for_gateway = wait_for_gateway;
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_custom_gateway_transceiver(
        mut self,
        gateway_transceiver: Box<dyn GatewayTransceiver + Send + Sync>,
    ) -> Self {
        self.custom_gateway_transceiver = Some(gateway_transceiver);
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_custom_topology_provider(
        mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        self.custom_topology_provider = Some(topology_provider);
        self
    }

    pub fn with_stored_topology<P: AsRef<Path>>(
        mut self,
        file: P,
    ) -> Result<Self, StatsCollectorError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    pub async fn run_service_provider(self) -> Result<(), StatsCollectorError> {
        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).
        let task_handle: TaskHandle = self.shutdown.map(Into::into).unwrap_or_default();

        // Connect to the mixnet
        let mixnet_client = crate::mixnet_client::create_mixnet_client(
            &self.config.base,
            task_handle
                .get_handle()
                .named("nym_sdk::MixnetClient[STATS]"),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
        .await?;

        let self_address = *mixnet_client.nym_address();

        let report_storage =
            ClientStatsStorage::init(self.config.storage_paths.client_reports_database).await?;

        let mixnet_listener =
            crate::mixnet_listener::MixnetListener::new(mixnet_client, report_storage, task_handle);

        log::info!("The address of this client is: {self_address}");
        log::info!("All systems go. Press CTRL-C to stop the server.");

        mixnet_listener.run().await
    }
}
