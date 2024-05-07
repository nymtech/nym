// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_utils::utils;
use nym_crypto::asymmetric::identity;
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

pub async fn execute(args: Args, client: SigningClient) -> anyhow::Result<()> {
    let loaded = CommonConfigsWrapper::try_load(args.client_config)?;

    if let Ok(id) = loaded.try_get_id() {
        println!("loaded config file for client '{id}'");
    }

    let Ok(credentials_store) = loaded.try_get_credentials_store() else {
        bail!("the loaded config does not have a credentials store information")
    };

    let Ok(private_id_key) = loaded.try_get_private_id_key() else {
        bail!("the loaded config does not have a public id key information")
    };

    println!(
        "using credentials store at '{}'",
        credentials_store.display()
    );

    let persistent_storage = initialise_persistent_storage(credentials_store).await;
    let private_id_key: identity::PrivateKey = nym_pemstore::load_key(private_id_key)?;
    utils::issue_credential(
        &client,
        &private_id_key.to_bytes(),
        &persistent_storage,
        args.recovery_dir,
    )
    .await?;

    Ok(())
}
