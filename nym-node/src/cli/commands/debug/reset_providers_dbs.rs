// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::helpers::load_ed25519_identity_public_key;
use crate::node::ServiceProvidersData;
use nym_network_requester::{CustomGatewayDetails, GatewayDetails, GatewayRegistration};
use std::fs;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    let config = try_load_current_config(args.config.config_path()).await?;

    let public_key = load_ed25519_identity_public_key(
        &config.storage_paths.keys.public_ed25519_identity_key_file,
    )?;

    let storage_paths = &config.service_providers.storage_paths;
    for db_path in [
        &storage_paths.authenticator.gateway_registrations,
        &storage_paths.ip_packet_router.gateway_registrations,
        &storage_paths.network_requester.gateway_registrations,
    ] {
        fs::remove_file(db_path)?;
        let gateway_details: GatewayRegistration =
            GatewayDetails::Custom(CustomGatewayDetails::new(public_key)).into();
        ServiceProvidersData::initialise_client_gateway_storage(db_path, &gateway_details).await?;
    }
    Ok(())
}
