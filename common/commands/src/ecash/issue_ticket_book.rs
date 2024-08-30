// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::CommonConfigsWrapper;
use anyhow::{anyhow, bail};
use clap::ArgGroup;
use clap::Parser;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_storage::storage::Storage;
use nym_credential_utils::utils;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::identity;
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[derive(Debug, Parser)]
#[clap(
    group(ArgGroup::new("output").required(true)),
)]
pub struct Args {
    /// Specify which type of ticketbook should be issued
    #[clap(long, default_value_t = TicketType::V1MixnetEntry)]
    pub(crate) ticketbook_type: TicketType,

    /// Config file of the client that is supposed to use the credential.
    #[clap(long, group = "output")]
    pub(crate) client_config: Option<PathBuf>,

    /// Output file for the ticketbook
    #[clap(long, group = "output", requires = "bs58_encoded_client_secret")]
    pub(crate) output_file: Option<PathBuf>,

    /// Specifies whether the output file should use binary or bs58 encoded data
    #[clap(long, requires = "output_file")]
    pub(crate) bs58_output: bool,

    /// Specifies whether the file output should contain expiration date signatures
    #[clap(long, requires = "output_file")]
    pub(crate) include_expiration_date_signatures: bool,

    /// Specifies whether the file output should contain coin index signatures
    #[clap(long, requires = "output_file")]
    pub(crate) include_coin_index_signatures: bool,

    /// Specifies whether the file output should contain master verification key
    #[clap(long, requires = "output_file")]
    pub(crate) include_master_verification_key: bool,

    /// Secret value that's used for deriving underlying ecash keypair
    #[clap(long)]
    pub(crate) bs58_encoded_client_secret: Option<String>,
}

async fn issue_client_ticketbook(
    config_path: PathBuf,
    ticketbook_type: TicketType,
    client: SigningClient,
) -> anyhow::Result<()> {
    let loaded = CommonConfigsWrapper::try_load(config_path)?;

    if let Ok(id) = loaded.try_get_id() {
        println!("loaded config file for client '{id}'");
    }

    let Ok(credentials_store) = loaded.try_get_credentials_store() else {
        bail!("the loaded config does not have a credentials store information")
    };

    let Ok(private_id_key) = loaded.try_get_private_id_key() else {
        bail!("the loaded config does not have a public id key information")
    };

    println!(
        "using credentials store at '{}'",
        credentials_store.display()
    );

    let persistent_storage = initialise_persistent_storage(credentials_store).await;
    let private_id_key: identity::PrivateKey = nym_pemstore::load_key(private_id_key)?;
    utils::issue_credential(
        &client,
        &persistent_storage,
        &private_id_key.to_bytes(),
        ticketbook_type,
    )
    .await?;

    Ok(())
}

async fn issue_to_file(args: Args, client: SigningClient) -> anyhow::Result<()> {
    // those MUST HAVE been specified; clap ensures it
    let output_file = args.output_file.unwrap();
    let secret = bs58::decode(&args.bs58_encoded_client_secret.unwrap()).into_vec()?;

    let temp_credential_store_file = NamedTempFile::new()?;
    let credential_store_path = temp_credential_store_file.into_temp_path();

    let credentials_store = initialise_persistent_storage(credential_store_path).await;

    utils::issue_credential(&client, &credentials_store, &secret, args.ticketbook_type).await?;

    let ticketbook = credentials_store
        .get_next_unspent_usable_ticketbook(0)
        .await?
        .ok_or(anyhow!("we just issued a ticketbook, it must be present!"))?
        .ticketbook;

    let expiration_date = ticketbook.expiration_date();
    let epoch_id = ticketbook.epoch_id();

    let mut exported = ticketbook.begin_export();

    if args.include_expiration_date_signatures {
        let signatures = credentials_store
            .get_expiration_date_signatures(expiration_date)
            .await?
            .ok_or(anyhow!("missing expiration date signatures!"))?;

        exported.with_expiration_date_signatures(&AggregatedExpirationDateSignatures {
            epoch_id,
            expiration_date,
            signatures,
        });
    }

    if args.include_coin_index_signatures {
        let signatures = credentials_store
            .get_coin_index_signatures(epoch_id)
            .await?
            .ok_or(anyhow!("missing coin index signatures!"))?;
        exported.with_coin_index_signatures(&AggregatedCoinIndicesSignatures {
            epoch_id,
            signatures,
        });
    }

    if args.include_master_verification_key {
        let key = credentials_store
            .get_master_verification_key(epoch_id)
            .await?
            .ok_or(anyhow!("missing master verification key!"))?;

        exported.with_master_verification_key(&EpochVerificationKey { epoch_id, key });
    }

    let data = exported.pack().data;

    if args.bs58_output {
        fs::write(output_file, bs58::encode(&data).into_string())?;
    } else {
        fs::write(output_file, &data)?;
    }

    Ok(())
}

pub async fn execute(args: Args, client: SigningClient) -> anyhow::Result<()> {
    if let Some(client_config) = args.client_config {
        return issue_client_ticketbook(client_config, args.ticketbook_type, client).await;
    }

    issue_to_file(args, client).await
}
