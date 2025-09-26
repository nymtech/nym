// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bandwidth_controller::{BandwidthController, BandwidthTicketProvider};
use nym_credential_storage::ephemeral_storage::EphemeralCredentialStorage;
use nym_sdk::{
    mixnet::{MixnetClient, MixnetClientBuilder},
    NymNetworkDetails,
};
use nym_validator_client::{
    nyxd::{Config as NyxdClientConfig, NyxdClient},
    QueryHttpRpcNyxdClient,
};
use std::time::Duration;

use crate::{config::RegistrationClientConfig, error::RegistrationClientError, RegistrationClient};
use config::BuilderConfig;

pub(crate) mod config;

pub(crate) const MIXNET_CLIENT_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RegistrationClientBuilder {
    pub config: BuilderConfig,
}

impl RegistrationClientBuilder {
    pub fn new(config: BuilderConfig) -> Self {
        Self { config }
    }

    pub async fn build(self) -> Result<RegistrationClient, RegistrationClientError> {
        let storage = self.config.setup_storage().await?;
        let config = RegistrationClientConfig {
            entry: self.config.entry_node,
            exit: self.config.exit_node,
            two_hops: self.config.two_hops,
            data_path: self.config.data_path.clone(),
        };
        let cancel_token = self.config.cancel_token.clone();

        let nyxd_client = get_nyxd_client(&self.config.network_env)?;

        let (mixnet_client, bandwidth_controller): (
            MixnetClient,
            Box<dyn BandwidthTicketProvider>,
        ) = if let Some((mixnet_client_storage, credential_storage)) = storage {
            let builder = MixnetClientBuilder::new_with_storage(mixnet_client_storage);
            let mixnet_client = tokio::time::timeout(
                MIXNET_CLIENT_STARTUP_TIMEOUT,
                self.config.build_and_connect_mixnet_client(builder),
            )
            .await??;
            let bandwidth_controller =
                Box::new(BandwidthController::new(credential_storage, nyxd_client));
            (mixnet_client, bandwidth_controller)
        } else {
            let builder = MixnetClientBuilder::new_ephemeral();
            let mixnet_client = tokio::time::timeout(
                MIXNET_CLIENT_STARTUP_TIMEOUT,
                self.config.build_and_connect_mixnet_client(builder),
            )
            .await??;
            let bandwidth_controller = Box::new(BandwidthController::new(
                EphemeralCredentialStorage::default(),
                nyxd_client,
            ));
            (mixnet_client, bandwidth_controller)
        };
        let mixnet_client_address = *mixnet_client.nym_address();

        Ok(RegistrationClient {
            mixnet_client,
            config,
            cancel_token,
            mixnet_client_address,
            bandwidth_controller,
        })
    }
}

// temporary while we use the legacy bandwidth-controller
fn get_nyxd_client(
    network: &NymNetworkDetails,
) -> Result<QueryHttpRpcNyxdClient, RegistrationClientError> {
    let config = NyxdClientConfig::try_from_nym_network_details(network)
        .map_err(RegistrationClientError::FailedToCreateNyxdClientConfig)?;
    let nyxd_url = network
        .endpoints
        .first()
        .map(|ep| ep.nyxd_url())
        .ok_or(RegistrationClientError::InvalidNyxdUrl)?;

    NyxdClient::connect(config, nyxd_url.as_str())
        .map_err(RegistrationClientError::FailedToConnectUsingNyxdClient)
}
