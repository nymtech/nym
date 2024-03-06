// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
    /// Explicitly provide the encoded credential data (as base58)
    #[clap(long, group = "cred_data", value_parser = parse_encoded_credential_data)]
    pub(crate) credential_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary credential data
    #[clap(long, group = "cred_data")]
    pub(crate) credential_path: Option<PathBuf>,

    /// Specifies path to the credentials storage
    #[clap(long)]
    pub credentials_store_path: PathBuf,

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true)]
    pub(crate) version: Option<u8>,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let credentials_store =
        nym_credential_storage::initialise_persistent_storage(args.credentials_store_path).await;

    let raw_credential = match args.credential_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            #[allow(clippy::unwrap_used)]
            fs::read(args.credential_path.unwrap())?
        }
    };

    import_credential(credentials_store, raw_credential, args.version).await?;
    Ok(())
}
