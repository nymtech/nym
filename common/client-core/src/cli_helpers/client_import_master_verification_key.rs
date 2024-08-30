// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "cli")]
fn parse_encoded_key_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[cfg_attr(feature = "cli",
    clap(
        group(clap::ArgGroup::new("key_data").required(true)),
    ))
]
pub struct CommonClientImportMasterVerificationKeyArgs {
    /// Id of client that is going to import the key
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Config file of the client that is supposed to use the key.
    #[cfg_attr(feature = "cli", clap(long))]
    pub(crate) client_config: PathBuf,

    /// Explicitly provide the encoded key data (as base58)
    #[cfg_attr(feature = "cli", clap(long, group = "key_data", value_parser = parse_encoded_key_data))]
    pub(crate) key_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary key data
    #[cfg_attr(feature = "cli", clap(long, group = "key_data"))]
    pub(crate) key_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
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
        nym_credential_storage::initialise_persistent_storage(&paths.credentials_database).await;

    let version = common_args.version;
    let raw_key = match common_args.key_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            fs::read(common_args.key_path.unwrap())?
        }
    };

    nym_id::import_master_verification_key(credentials_store, raw_key, version).await?;

    Ok(())
}
