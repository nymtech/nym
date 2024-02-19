// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::error::Socks5ClientError;
use clap::ArgGroup;
use log::{error, info};
use nym_credential_storage::models::StorableIssuedCredential;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::issued::BandwidthCredentialIssuedDataVariant;
use nym_credentials::IssuedBandwidthCredential;
use std::fs;
use std::path::PathBuf;
use time::OffsetDateTime;
use zeroize::Zeroizing;

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
    #[clap(long, hide = true, default_value_t = 1)]
    pub(crate) version: u8,
}

pub(crate) async fn execute(args: Args) -> Result<(), Socks5ClientError> {
    let config = try_load_current_config(&args.id)?;

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
    let raw_credential = Zeroizing::new(raw_credential);

    // we're unpacking the data in order to make sure it's valid
    // and to extract relevant metadata for storage purposes
    let credential = match args.version {
        1 => Zeroizing::new(
            IssuedBandwidthCredential::unpack_v1(&raw_credential).map_err(|source| {
                Socks5ClientError::CredentialDeserializationFailure {
                    storage_revision: 1,
                    source,
                }
            })?,
        ),
        other => panic!("unknown credential serialization version {other}"),
    };

    info!("importing {}", credential.typ());
    match credential.variant_data() {
        BandwidthCredentialIssuedDataVariant::Voucher(voucher_info) => {
            info!("with value of {}", voucher_info.value())
        }
        BandwidthCredentialIssuedDataVariant::FreePass(freepass_info) => {
            info!("with expiry at {}", freepass_info.expiry_date());
            if freepass_info.expired() {
                error!("the free pass has already expired!");

                // technically we can import it, but the gateway will just reject it so what's the point
                return Err(Socks5ClientError::ExpiredCredentialImport {
                    expiration: freepass_info.expiry_date(),
                });
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

    credentials_store.insert_issued_credential(storable).await?;
    Ok(())
}
