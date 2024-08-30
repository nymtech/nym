// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::ArgGroup;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_id::import_credential::import_expiration_date_signatures;
use std::fs;
use std::path::PathBuf;

fn parse_encoded_signatures_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[derive(Debug, Parser)]
#[clap(
    group(ArgGroup::new("signatures_data").required(true)),
)]
pub struct Args {
    /// Config file of the client that is supposed to use the signatures.
    #[clap(long)]
    pub(crate) client_config: PathBuf,

    /// Explicitly provide the encoded signatures data (as base58)
    #[clap(long, group = "signatures_data", value_parser = parse_encoded_signatures_data)]
    pub(crate) signatures_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary signatures data
    #[clap(long, group = "signatures_data")]
    pub(crate) signatures_path: Option<PathBuf>,

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true)]
    pub(crate) version: Option<u8>,
}

impl Args {
    fn signatures_data(self) -> anyhow::Result<Vec<u8>> {
        let data = match self.signatures_data {
            Some(data) => data,
            None => {
                // SAFETY: one of those arguments must have been set
                #[allow(clippy::unwrap_used)]
                fs::read(self.signatures_path.unwrap())?
            }
        };
        Ok(data)
    }
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    let loaded = CommonConfigsWrapper::try_load(&args.client_config)?;

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

    let version = args.version;
    let raw_signatures = args.signatures_data()?;

    import_expiration_date_signatures(credentials_store, raw_signatures, version).await?;

    Ok(())
}
