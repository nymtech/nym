// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::CommonConfigsWrapper;
use anyhow::bail;
use clap::ArgGroup;
use clap::Parser;
use log::{error, info};
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_storage::models::StorableIssuedCredential;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::issued::BandwidthCredentialIssuedDataVariant;
use nym_credentials::IssuedBandwidthCredential;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroizing;

fn parse_encoded_credential_data(raw: &str) -> bs58::decode::Result<Vec<u8>> {
    bs58::decode(raw).into_vec()
}

#[derive(Debug, Parser)]
#[clap(group(ArgGroup::new("cred_data").required(true)))]
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

    // currently hidden as there exists only a single serialization standard
    #[clap(long, hide = true, default_value_t = 1)]
    pub(crate) version: u8,
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

    let raw_credential = match args.credential_data {
        Some(data) => data,
        None => {
            // SAFETY: one of those arguments must have been set
            fs::read(args.credential_path.unwrap())?
        }
    };
    let raw_credential = Zeroizing::new(raw_credential);

    // we're unpacking the data in order to make sure it's valid
    // and to extract relevant metadata for storage purposes
    let credential = match args.version {
        1 => Zeroizing::new(IssuedBandwidthCredential::unpack_v1(&raw_credential)?),
        other => panic!("unknown credential serialization version {other}"),
    };
    let persistent_storage = initialise_persistent_storage(credentials_store).await;

    info!("importing {}", credential.typ());
    match credential.variant_data() {
        BandwidthCredentialIssuedDataVariant::Voucher(voucher_info) => {
            info!("with value of {}", voucher_info.value())
        }
        BandwidthCredentialIssuedDataVariant::FreePass(freepass_info) => {
            info!("with expiry at {}", freepass_info.expiry_date());
            if freepass_info.expired() {
                error!("the free pass has already expired!");

                // technically we can, but the gateway will just reject it so what's the point
                bail!("can't import an expired free pass")
            }
        }
    }

    let storable = StorableIssuedCredential {
        serialization_revision: args.version,
        credential_data: &raw_credential,
        credential_type: credential.typ().to_string(),
        epoch_id: credential
            .epoch_id()
            .try_into()
            .expect("our epoch is has run over u32::MAX!"),
    };

    persistent_storage
        .insert_issued_credential(storable)
        .await?;
    Ok(())
}
