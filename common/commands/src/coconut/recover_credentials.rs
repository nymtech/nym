// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClient;
use crate::utils::ClientConfigCommonWrapper;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_utils::{recovery_storage, utils};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    /// Config file of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_config: PathBuf,

    /// Path to a directory used to store recovery files for unconsumed deposits
    #[clap(long)]
    pub(crate) recovery_dir: PathBuf,
}

pub async fn execute(args: Args, client: QueryClient) -> anyhow::Result<()> {
    let common_cfg = ClientConfigCommonWrapper::try_load(args.client_config)?;
    println!("loaded config file for client '{}'", common_cfg.client.id);

    let persistent_storage =
        initialise_persistent_storage(common_cfg.storage_paths.credentials_database).await;
    let recovery_storage = recovery_storage::RecoveryStorage::new(args.recovery_dir)?;

    let recovered =
        utils::recover_credentials(&client, &recovery_storage, &persistent_storage).await?;

    // TODO: denom?
    println!("recovered {recovered} worth of credentials");
    Ok(())
}
