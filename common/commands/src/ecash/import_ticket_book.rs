// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::ArgGroup;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_id::import_credential::import_full_ticketbook;
use nym_id::import_standalone_ticketbook;
use std::fs;
use std::path::PathBuf;

fn parse_encoded_credential_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[derive(Debug, Parser)]
#[clap(
    group(ArgGroup::new("cred_data").required(true)),
    group(ArgGroup::new("type").required(true)),
)]
pub struct Args {
    /// Config file of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_config: PathBuf,

    /// Explicitly provide the encoded credential data (as base58)
    #[clap(long, group = "cred_data", value_parser = parse_encoded_credential_data)]
    pub(crate) credential_data: Option<Vec<u8>>,

    /// Specifies the path to file containing binary credential data
    #[clap(long, group = "cred_data")]
    pub(crate) credential_path: Option<PathBuf>,

    /// Specifies whether we're attempting to import a standalone ticketbook (i.e. serialised `IssuedTicketBook`)
    #[clap(long, group = "type")]
    pub(crate) standalone: bool,

    /// Specifies whether we're attempting to import full ticketboot
    /// (i.e. one that **might** contain required global signatures; that is serialised `ImportableTicketBook`)
    #[clap(long, group = "type")]
    pub(crate) full: bool,

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true)]
    pub(crate) version: Option<u8>,
}

impl Args {
    fn credential_data(self) -> anyhow::Result<Vec<u8>> {
        let data = match self.credential_data {
            Some(data) => data,
            None => {
                // SAFETY: one of those arguments must have been set
                #[allow(clippy::unwrap_used)]
                fs::read(self.credential_path.unwrap())?
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
    let standalone = args.standalone;
    let full = args.full;
    let raw_credential = args.credential_data()?;

    if standalone {
        import_standalone_ticketbook(credentials_store, raw_credential, version).await?;
    } else {
        // sanity check; clap should have ensured it
        assert!(full);
        import_full_ticketbook(credentials_store, raw_credential, version).await?;
    }

    Ok(())
}
