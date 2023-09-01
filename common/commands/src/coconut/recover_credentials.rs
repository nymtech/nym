// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClient;
use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
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
    let loaded = CommonConfigsWrapper::try_load(args.client_config)?;

    if let Ok(id) = loaded.try_get_id() {
        println!("loaded config file for client '{id}'");
    }

    let Ok(credentials_store) = loaded.try_get_credentials_store() else {
        bail!("the loaded config does not have a credentials store information")
    };

    println!(
        "using credentials store at '{}'",
        credentials_store.display()
    );

    let persistent_storage = initialise_persistent_storage(credentials_store).await;
    let recovery_storage = recovery_storage::RecoveryStorage::new(args.recovery_dir)?;

    let recovered =
        utils::recover_credentials(&client, &recovery_storage, &persistent_storage).await?;

    // TODO: denom?
    println!("recovered {recovered} worth of credentials");
    Ok(())
}
