// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::cli::peer_handler::DummyHandler;
use crate::cli::{override_config, OverrideConfig};
use crate::cli::{try_load_current_config, version_check};
use clap::Args;
use log::error;
use nym_authenticator::error::AuthenticatorError;
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_gateway_storage::PersistentStorage;
use nym_task::TaskHandle;
use nym_wireguard::WireguardGatewayData;
use rand::rngs::OsRng;

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Clone)]
pub(crate) struct Run {
    #[command(flatten)]
    common_args: CommonClientRunArgs,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: None,
            nyxd_urls: run_config.common_args.nyxd_urls,
            enabled_credentials_mode: run_config.common_args.enabled_credentials_mode,
        }
    }
}

pub(crate) async fn execute(args: &Run) -> Result<(), AuthenticatorError> {
    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));
    log::debug!("Using config: {:#?}", config);

    if !version_check(&config) {
        error!("failed the local version check");
        return Err(AuthenticatorError::FailedLocalVersionCheck);
    }

    log::info!("Starting authenticator service provider");
    let (wireguard_gateway_data, peer_rx) = WireguardGatewayData::new(
        config.authenticator.clone().into(),
        Arc::new(KeyPair::new(&mut OsRng)),
    );
    let task_handler = TaskHandle::default();
    let handler = DummyHandler::new(peer_rx, task_handler.fork("peer-handler"));
    tokio::spawn(async move {
        handler.run().await;
    });

    let mut server = nym_authenticator::Authenticator::<PersistentStorage>::new(
        config,
        wireguard_gateway_data,
        vec![],
    );
    if let Some(custom_mixnet) = &args.common_args.custom_mixnet {
        server = server.with_stored_topology(custom_mixnet)?
    }

    server.run_service_provider().await
}
