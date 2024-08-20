// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_utils::utils;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::identity;
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs::create_dir_all;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    /// Specify which type of ticketbook should be issued
    #[clap(long, default_value_t = TicketType::default())]
    pub(crate) ticketbook_type: TicketType,

    /// Config file of the client that is supposed to use the credential.
    #[clap(long, group = "output")]
    pub(crate) client_config: Option<PathBuf>,

    /// Path to the dedicated credential storage database
    #[clap(long, group = "output")]
    pub(crate) credential_storage: Option<PathBuf>,
}

async fn issue_client_ticketbook(
    cfg: PathBuf,
    typ: TicketType,
    client: SigningClient,
) -> anyhow::Result<()> {
    let loaded = CommonConfigsWrapper::try_load(cfg)?;

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
        &persistent_storage,
        &private_id_key.to_bytes(),
        typ,
    )
    .await?;

    Ok(())
}

async fn issue_standalone_ticketbook(
    credentials_store: PathBuf,
    typ: TicketType,
    client: SigningClient,
) -> anyhow::Result<()> {
    println!("attempting to issue a standalone ticketbook");

    let mut rng = OsRng;
    let mut random_seed = [0u8; 32];
    rng.fill_bytes(&mut random_seed);

    if let Some(parent) = credentials_store.parent() {
        create_dir_all(parent)?;
    }

    let persistent_storage = initialise_persistent_storage(credentials_store).await;
    utils::issue_credential(&client, &persistent_storage, &random_seed, typ).await?;

    Ok(())
}

pub async fn execute(args: Args, client: SigningClient) -> anyhow::Result<()> {
    match (args.client_config, args.credential_storage) {
        (Some(cfg), None) => issue_client_ticketbook(cfg, args.ticketbook_type, client).await,
        (None, Some(storage)) => {
            issue_standalone_ticketbook(storage, args.ticketbook_type, client).await
        }
        _ => unreachable!("clap should have made this branch impossible to reach!"),
    }
}
