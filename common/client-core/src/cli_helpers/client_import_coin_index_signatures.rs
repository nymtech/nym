// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "cli")]
fn parse_encoded_signatures_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[cfg_attr(feature = "cli",
    clap(
        group(clap::ArgGroup::new("sig_data").required(true)),
    ))
]
pub struct CommonClientImportCoinIndexSignaturesArgs {
    /// Id of client that is going to import the signatures
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Config file of the client that is supposed to use the signatures.
    #[cfg_attr(feature = "cli", clap(long))]
    pub(crate) client_config: PathBuf,

    /// Explicitly provide the encoded signatures data (as base58)
    #[cfg_attr(feature = "cli", clap(long, group = "sig_data", value_parser = parse_encoded_signatures_data))]
    pub(crate) signatures_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary signatures data
    #[cfg_attr(feature = "cli", clap(long, group = "sig_data"))]
    pub(crate) signatures_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub(crate) version: Option<u8>,
}

pub async fn import_coin_index_signatures<C, A>(args: A) -> Result<(), C::Error>
where
    A: Into<CommonClientImportCoinIndexSignaturesArgs>,
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
    let raw_key = match common_args.signatures_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            fs::read(common_args.signatures_path.unwrap())?
        }
    };

    nym_id::import_coin_index_signatures(credentials_store, raw_key, version).await?;

    Ok(())
}
