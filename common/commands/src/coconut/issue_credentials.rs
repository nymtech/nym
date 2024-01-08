// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_utils::utils;
use nym_validator_client::nyxd::Coin;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    /// Config file of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_config: PathBuf,

    /// The amount of utokens the credential will hold.
    #[clap(long, default_value = "0")]
    pub(crate) amount: u64,

    /// Path to a directory used to store recovery files for unconsumed deposits
    #[clap(long)]
    pub(crate) recovery_dir: PathBuf,
}

pub async fn execute(args: Args, client: SigningClient) -> anyhow::Result<()> {
    if args.amount == 0 {
        bail!("did not specify credential amount")
    }

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

    let denom = &client.current_chain_details().mix_denom.base;
    let coin = Coin::new(args.amount as u128, denom);

    let persistent_storage = initialise_persistent_storage(credentials_store).await;
    utils::issue_credential(&client, coin, &persistent_storage, args.recovery_dir).await?;

    Ok(())
}
