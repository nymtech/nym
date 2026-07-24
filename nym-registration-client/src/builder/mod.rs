// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use nym_sdk::mixnet::{EventSender, MixnetClientBuilder};

use crate::{
    RegistrationClient,
    clients::{LpBasedRegistrationClient, MixnetBasedRegistrationClient},
    config::RegistrationMode,
    error::RegistrationClientError,
};
use config::BuilderConfig;

pub(crate) mod config;

pub struct RegistrationClientBuilder {
    pub config: BuilderConfig,
}

impl RegistrationClientBuilder {
    pub fn new(config: BuilderConfig) -> Self {
        Self { config }
    }

    pub fn use_lp(&self) -> bool {
        let lp_enabled = self.config.enable_lp_registration;
        let lp_info_available = self.config.entry_node.node.lp_data.is_some()
            && self.config.exit_node.node.lp_data.is_some();
        // To remove when LP supports Mixnet registration
        let wireguard_mode = self.config.mode == RegistrationMode::Wireguard;
        let use_lp = lp_enabled && lp_info_available && wireguard_mode;
        if !use_lp && lp_enabled {
            tracing::warn!(
                "LP is enabled but can't be used: Missing LP information: {lp_info_available}, wireguard mode: {wireguard_mode}"
            );
        }
        use_lp
    }

    pub async fn build(self) -> Result<RegistrationClient, RegistrationClientError> {
        if self.use_lp() {
            tracing::debug!("Using LP for registration");
            Ok(RegistrationClient::Lp(Box::new(self.build_lp().await?)))
        } else {
            tracing::debug!("Using Mixnet for registration");
            Ok(RegistrationClient::Mixnet(Box::new(
                self.build_mixnet().await?,
            )))
        }
    }

    pub(crate) async fn build_mixnet(
        self,
    ) -> Result<MixnetBasedRegistrationClient, RegistrationClientError> {
        let storage = self.config.setup_mixnet_client_storage().await?;
        let config = self.config.registration_client_config();
        let cancel_token = self.config.cancel_token.clone();
        let (event_tx, event_rx) = mpsc::unbounded();

        let mixnet_client_startup_timeout = self.config.mixnet_client_startup_timeout;

        let bc_request_sender = self.config.bandwidth_request_sender.clone();

        let builder =
            MixnetClientBuilder::new_with_storage(storage).event_tx(EventSender(event_tx));

        let mixnet_client = tokio::time::timeout(
            mixnet_client_startup_timeout,
            self.config.build_and_connect_mixnet_client(builder),
        )
        .await
        .inspect_err(|_| {
            tracing::warn!(
                "mixnet client connection timed out after {:?}",
                mixnet_client_startup_timeout
            )
        })?
        .inspect_err(|e| tracing::warn!("mixnet build/connect error: {e}"))?;

        let mixnet_client_address = *mixnet_client.nym_address();

        Ok(MixnetBasedRegistrationClient {
            mixnet_client,
            config,
            cancel_token,
            mixnet_client_address,
            bandwidth_provider: Box::new(bc_request_sender),
            event_rx,
        })
    }

    async fn build_lp(self) -> Result<LpBasedRegistrationClient, RegistrationClientError> {
        let config = self.config.registration_client_config();
        let bc_request_sender = self.config.bandwidth_request_sender;

        Ok(LpBasedRegistrationClient {
            config,
            bandwidth_provider: Box::new(bc_request_sender),
            cancel_token: self.config.cancel_token.clone(),
        })
    }
}
