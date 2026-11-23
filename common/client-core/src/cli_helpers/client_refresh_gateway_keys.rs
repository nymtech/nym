// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::types::RefreshedGatewayKeys;
use crate::cli_helpers::{CliClient, CliClientConfig};
use crate::client::base_client::non_wasm_helpers::setup_fs_gateways_storage;
use crate::client::key_manager::persistence::{OnDiskKeys, Passphrase};
use crate::init::setup_gateway;
use crate::init::types::GatewaySetup;
use nym_crypto::asymmetric::ed25519;

#[derive(clap::Args, Debug, Clone)]
pub struct CommonClientRefreshGatewayKeysArgs {
    /// Id of the client whose keys at a gateway should be refreshed.
    #[clap(long)]
    pub id: String,

    /// Id of the gateway to register with again under new keys.
    /// If unspecified, the active gateway is used.
    #[clap(long, alias = "gateway")]
    pub gateway_id: Option<ed25519::PublicKey>,
}

pub async fn refresh_gateway_keys<C, A>(
    args: A,
    key_passphrase: Option<Passphrase>,
    database_key: Option<&nym_sqlcipher::DatabaseKey>,
) -> Result<RefreshedGatewayKeys, C::Error>
where
    A: AsRef<CommonClientRefreshGatewayKeysArgs>,
    C: CliClient,
{
    let common_args = args.as_ref();

    let config = C::try_load_current_config(&common_args.id).await?;
    let paths = config.common_paths();

    let key_store = OnDiskKeys::with_passphrase(paths.keys.clone(), key_passphrase);
    let details_store =
        setup_fs_gateways_storage(&paths.gateway_registrations, database_key).await?;

    let gateway_setup = GatewaySetup::RefreshKeys {
        gateway_id: common_args.gateway_id.map(|id| id.to_base58_string()),
        #[cfg(unix)]
        connection_fd_callback: None,
    };
    let init_details = setup_gateway(gateway_setup, &key_store, &details_store).await?;

    Ok(RefreshedGatewayKeys {
        gateway: init_details.gateway_id(),
        address: init_details.client_address(),
    })
}
