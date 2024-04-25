// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use crate::client::base_client::non_wasm_helpers::setup_fs_gateways_storage;
use crate::client::base_client::storage::helpers::set_active_gateway;
use nym_crypto::asymmetric::identity;

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct CommonClientSwitchGatewaysArgs {
    /// Id of client we want to list gateways for.
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Id of the gateway we want to switch to.
    #[cfg_attr(feature = "cli", clap(long))]
    pub gateway_id: identity::PublicKey,
}

pub async fn switch_gateway<C, A>(args: A) -> Result<(), C::Error>
where
    A: AsRef<CommonClientSwitchGatewaysArgs>,
    C: CliClient,
{
    let common_args = args.as_ref();
    let id = &common_args.id;

    let config = C::try_load_current_config(id).await?;
    let paths = config.common_paths();

    let details_store = setup_fs_gateways_storage(&paths.gateway_registrations).await?;

    set_active_gateway(&details_store, &common_args.gateway_id.to_base58_string()).await?;

    Ok(())
}
