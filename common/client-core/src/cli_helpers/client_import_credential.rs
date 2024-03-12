// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use std::fs;
use std::path::PathBuf;

fn parse_encoded_credential_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[cfg_attr(feature = "cli", clap(group(clap::ArgGroup::new("cred_data").required(true))))]
#[derive(Debug, Clone)]
pub struct CommonClientImportCredentialArgs {
    /// Id of client that is going to import the credential
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Explicitly provide the encoded credential data (as base58)
    #[cfg_attr(feature = "cli", clap(long, group = "cred_data", value_parser = parse_encoded_credential_data))]
    pub(crate) credential_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary credential data
    #[cfg_attr(feature = "cli", clap(long, group = "cred_data"))]
    pub(crate) credential_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub(crate) version: Option<u8>,
}

pub async fn import_credential<C, A>(args: A) -> Result<(), C::Error>
where
    A: Into<CommonClientImportCredentialArgs>,
    C: CliClient,
    C::Error: From<std::io::Error> + From<nym_id::NymIdError>,
{
    let common_args = args.into();
    let id = &common_args.id;

    let config = C::try_load_current_config(id).await?;
    let paths = config.common_paths();

    let credentials_store =
        nym_credential_storage::initialise_persistent_storage(&paths.credentials_database).await;

    let raw_credential = match common_args.credential_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            fs::read(common_args.credential_path.unwrap())?
        }
    };

    nym_id::import_credential(credentials_store, raw_credential, common_args.version).await?;
    Ok(())
}
