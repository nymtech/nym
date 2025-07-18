// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::peer_handler::DummyHandler;
use crate::cli::try_load_current_config;
use crate::cli::{override_config, OverrideConfig};
use clap::Args;
use nym_authenticator::error::AuthenticatorError;
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_task::TaskHandle;
use nym_wireguard::WireguardGatewayData;
use rand::rngs::OsRng;
use std::sync::Arc;

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
    log::debug!("Using config: {config:#?}");

    log::info!("Starting authenticator service provider");
    let (wireguard_gateway_data, peer_rx) = WireguardGatewayData::new(
        config.authenticator.clone().into(),
        Arc::new(KeyPair::new(&mut OsRng)),
    );
    let task_handler = TaskHandle::default();
    let handler = DummyHandler::new(peer_rx, task_handler.fork("peer_handler"));
    tokio::spawn(async move {
        handler.run().await;
    });

    let mut server = nym_authenticator::Authenticator::new(config, wireguard_gateway_data, vec![]);
    if let Some(custom_mixnet) = &args.common_args.custom_mixnet {
        server = server.with_stored_topology(custom_mixnet)?
    }

    server.run_service_provider().await
}
