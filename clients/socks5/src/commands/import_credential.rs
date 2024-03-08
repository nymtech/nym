// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::error::Socks5ClientError;
use clap::ArgGroup;
use nym_id::import_credential;
use std::fs;
use std::path::PathBuf;

fn parse_encoded_credential_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[derive(clap::Args)]
#[clap(group(ArgGroup::new("cred_data").required(true)))]
pub(crate) struct Args {
    /// Id of client that is going to import the credential
    #[clap(long)]
    pub id: String,

    /// Explicitly provide the encoded credential data (as base58)
    #[clap(long, group = "cred_data", value_parser = parse_encoded_credential_data)]
    pub(crate) credential_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary credential data
    #[clap(long, group = "cred_data")]
    pub(crate) credential_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true)]
    pub(crate) version: Option<u8>,
}

pub(crate) async fn execute(args: Args) -> Result<(), Socks5ClientError> {
    let config = try_load_current_config(&args.id).await?;

    let credentials_store = nym_credential_storage::initialise_persistent_storage(
        &config.storage_paths.common_paths.credentials_database,
    )
    .await;

    let raw_credential = match args.credential_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            fs::read(args.credential_path.unwrap())?
        }
    };

    import_credential(credentials_store, raw_credential, args.version).await?;
    Ok(())
}
