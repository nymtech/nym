// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use std::fs;
use std::path::PathBuf;

fn parse_encoded_key_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[derive(clap::Args)]
#[clap(group(clap::ArgGroup::new("key_data_group").required(true)))]
pub struct CommonClientImportMasterVerificationKeyArgs {
    /// Id of client that is going to import the key
    #[clap(long)]
    pub id: String,

    /// Config file of the client that is supposed to use the key.
    #[clap(long)]
    pub(crate) client_config: PathBuf,

    /// Explicitly provide the encoded key data (as base58)
    #[clap(long, group = "key_data_group", value_parser = parse_encoded_key_data)]
    pub(crate) key_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary key data
    #[clap(long, group = "key_data_group")]
    pub(crate) key_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true)]
    pub(crate) version: Option<u8>,
}

pub async fn import_master_verification_key<C, A>(args: A) -> Result<(), C::Error>
where
    A: Into<CommonClientImportMasterVerificationKeyArgs>,
    C: CliClient,
    C::Error: From<std::io::Error> + From<nym_id::NymIdError>,
{
    let common_args = args.into();
    let id = &common_args.id;

    let config = C::try_load_current_config(id).await?;
    let paths = config.common_paths();

    let credentials_store =
        nym_credential_storage::initialise_persistent_storage(&paths.credentials_database, None)
            .await;

    let version = common_args.version;
    let raw_key = match common_args.key_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            #[allow(clippy::unwrap_used)]
            fs::read(common_args.key_path.unwrap())?
        }
    };

    nym_id::import_master_verification_key(credentials_store, raw_key, version).await?;

    Ok(())
}
