// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClient;
use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_utils::utils;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    /// Config file of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_config: PathBuf,
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

    let recovered = utils::recover_deposits(&client, &persistent_storage).await?;

    println!("recovered {recovered} ticketbooks");
    Ok(())
}
