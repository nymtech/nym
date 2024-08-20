// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::ArgGroup;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_id::import_credential;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct CredentialDataWrapper(Vec<u8>);

impl FromStr for CredentialDataWrapper {
    type Err = bs58::decode::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        bs58::decode(s).into_vec().map(CredentialDataWrapper)
    }
}

#[derive(Debug, Parser)]
#[clap(group(ArgGroup::new("cred_data").required(true)))]
pub struct Args {
    /// Config file of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_config: PathBuf,

    /// Explicitly provide the encoded credential data (as base58)
    #[clap(long, group = "cred_data")]
    pub(crate) credential_data: Option<CredentialDataWrapper>,

    /// Specifies the path to file containing binary credential data
    #[clap(long, group = "cred_data")]
    pub(crate) credential_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true)]
    pub(crate) version: Option<u8>,
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
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
    let credentials_store = initialise_persistent_storage(credentials_store).await;

    let raw_credential = match args.credential_data {
        Some(data) => data.0,
        None => {
            // SAFETY: one of those arguments must have been set
            fs::read(args.credential_path.unwrap())?
        }
    };

    import_credential(credentials_store, raw_credential, args.version).await?;
    Ok(())
}
