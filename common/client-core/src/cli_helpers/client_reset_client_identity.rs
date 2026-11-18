// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use crate::client::base_client::non_wasm_helpers;
use crate::client::key_manager::persistence::Passphrase;

#[derive(clap::Args, Debug, Clone)]
pub struct CommonClientResetClientIdentityArgs {
    /// Id of the client to give a new identity.
    #[clap(long)]
    pub id: String,
}

pub async fn reset_client_identity<C, A>(
    args: A,
    key_passphrase: Option<Passphrase>,
    database_key: Option<&nym_sqlcipher::DatabaseKey>,
) -> Result<(), C::Error>
where
    A: AsRef<CommonClientResetClientIdentityArgs>,
    C: CliClient,
{
    let config = C::try_load_current_config(&args.as_ref().id).await?;
    non_wasm_helpers::reset_client_identity(config.common_paths(), key_passphrase, database_key)
        .await?;
    Ok(())
}
