// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};
use completions::ArgShell;
use rand::rngs::OsRng;
use std::str::FromStr;

use coconut_interface::{Base58, Parameters};
use credential_storage::storage::Storage;
use credential_storage::PersistentStorage;
use credentials::coconut::bandwidth::{BandwidthVoucher, TOTAL_ATTRIBUTES};
use credentials::coconut::utils::obtain_aggregate_signature;
use crypto::asymmetric::{encryption, identity};
use network_defaults::{NymNetworkDetails, VOUCHER_INFO};
use validator_client::nymd::tx::Hash;
use validator_client::{CoconutApiClient, Config};

use crate::client::Client;
use crate::error::{CredentialClientError, Result};
use crate::state::{KeyPair, State};

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Run the binary
    Run(Run),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

#[derive(Args)]
pub(crate) struct Run {
    /// Home directory of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_home_directory: std::path::PathBuf,

    /// The nymd URL that should be used
    #[clap(long)]
    pub(crate) nymd_url: String,

    /// A mnemonic for the account that buys the credential
    #[clap(long)]
    pub(crate) mnemonic: String,

    /// The amount of utokens the credential will hold
    #[clap(long)]
    pub(crate) amount: u64,
}

pub(crate) async fn deposit(nymd_url: &str, mnemonic: &str, amount: u64) -> Result<State> {
    let mut rng = OsRng;
    let signing_keypair = KeyPair::from(identity::KeyPair::new(&mut rng));
    let encryption_keypair = KeyPair::from(encryption::KeyPair::new(&mut rng));

    let client = Client::new(nymd_url, mnemonic);
    let tx_hash = client
        .deposit(
            amount,
            signing_keypair.public_key.clone(),
            encryption_keypair.public_key.clone(),
            None,
        )
        .await?;

    let state = State {
        amount,
        tx_hash: tx_hash.clone(),
        signing_keypair,
        encryption_keypair,
    };

    Ok(state)
}

pub(crate) async fn get_credential(state: &State, shared_storage: PersistentStorage) -> Result<()> {
    let network_details = NymNetworkDetails::new_from_env();
    let config = Config::try_from_nym_network_details(&network_details)?;
    let client = validator_client::Client::new_query(config)?;
    let coconut_api_clients = CoconutApiClient::all_coconut_api_clients(&client).await?;

    let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();
    let bandwidth_credential_attributes = BandwidthVoucher::new(
        &params,
        state.amount.to_string(),
        VOUCHER_INFO.to_string(),
        Hash::from_str(&state.tx_hash).map_err(|_| CredentialClientError::InvalidTxHash)?,
        identity::PrivateKey::from_base58_string(&state.signing_keypair.private_key)?,
        encryption::PrivateKey::from_base58_string(&state.encryption_keypair.private_key)?,
    );

    let signature = obtain_aggregate_signature(
        &params,
        &bandwidth_credential_attributes,
        &coconut_api_clients,
    )
    .await?;
    println!("Signature: {:?}", signature.to_bs58());
    shared_storage
        .insert_coconut_credential(
            state.amount.to_string(),
            VOUCHER_INFO.to_string(),
            bandwidth_credential_attributes.get_private_attributes()[0].to_bs58(),
            bandwidth_credential_attributes.get_private_attributes()[1].to_bs58(),
            signature.to_bs58(),
        )
        .await?;

    Ok(())
}
