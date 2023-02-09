// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{ArgGroup, Args, Subcommand};
use completions::ArgShell;
use rand::rngs::OsRng;
use std::str::FromStr;

use coconut_interface::{Base58, Parameters};
use credential_storage::storage::Storage;
use credential_storage::PersistentStorage;
use credentials::coconut::bandwidth::{BandwidthVoucher, TOTAL_ATTRIBUTES};
use credentials::coconut::utils::obtain_aggregate_signature;
use crypto::asymmetric::{encryption, identity};
use network_defaults::VOUCHER_INFO;
use validator_client::nyxd::traits::DkgQueryClient;
use validator_client::nyxd::tx::Hash;
use validator_client::nyxd::CosmWasmClient;
use validator_client::CoconutApiClient;

use crate::client::Client;
use crate::error::{CredentialClientError, Result};
use crate::recovery_storage::RecoveryStorage;
use crate::state::{KeyPair, State};

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Run the binary to obtain a credential
    Run(Run),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

#[derive(Args)]
#[clap(group(
ArgGroup::new("recov")
.required(true)
.args(&["amount", "recovery_mode"]),
))]
pub(crate) struct Run {
    /// Home directory of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_home_directory: std::path::PathBuf,

    /// The nyxd URL that should be used
    #[clap(long)]
    pub(crate) nyxd_url: String,

    /// A mnemonic for the account that buys the credential
    #[clap(long)]
    pub(crate) mnemonic: String,

    /// The amount of utokens the credential will hold. If recovery mode is enabled, this
    #[clap(long)]
    pub(crate) amount: u64,

    /// Path to a directory used to store recovery files for unconsumed deposits
    #[clap(long)]
    pub(crate) recovery_dir: std::path::PathBuf,

    /// Recovery mode, when enabled, tries to recover any deposit data dumped in recovery_dir
    #[clap(long)]
    pub(crate) recovery_mode: bool,
}

pub(crate) async fn deposit(nyxd_url: &str, mnemonic: &str, amount: u64) -> Result<State> {
    let mut rng = OsRng;
    let signing_keypair = KeyPair::from(identity::KeyPair::new(&mut rng));
    let encryption_keypair = KeyPair::from(encryption::KeyPair::new(&mut rng));
    let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();

    let client = Client::new(nyxd_url, mnemonic);
    let tx_hash = client
        .deposit(
            amount,
            signing_keypair.public_key.clone(),
            encryption_keypair.public_key.clone(),
            None,
        )
        .await?;

    let voucher = BandwidthVoucher::new(
        &params,
        amount.to_string(),
        VOUCHER_INFO.to_string(),
        Hash::from_str(&tx_hash).map_err(|_| CredentialClientError::InvalidTxHash)?,
        identity::PrivateKey::from_base58_string(&signing_keypair.private_key)?,
        encryption::PrivateKey::from_base58_string(&encryption_keypair.private_key)?,
    );

    let state = State {
        amount,
        voucher,
        params,
    };

    Ok(state)
}

pub(crate) async fn get_credential<C: Clone + CosmWasmClient + Send + Sync>(
    state: &State,
    client: validator_client::Client<C>,
    shared_storage: PersistentStorage,
) -> Result<()> {
    let epoch_id = client.nyxd.get_current_epoch().await?.epoch_id;
    let threshold = client
        .nyxd
        .get_current_epoch_threshold()
        .await?
        .ok_or(CredentialClientError::NoThreshold)?;
    let coconut_api_clients = CoconutApiClient::all_coconut_api_clients(&client, epoch_id).await?;

    let signature = obtain_aggregate_signature(
        &state.params,
        &state.voucher,
        &coconut_api_clients,
        threshold,
    )
    .await?;
    println!("Signature: {:?}", signature.to_bs58());
    shared_storage
        .insert_coconut_credential(
            state.amount.to_string(),
            VOUCHER_INFO.to_string(),
            state.voucher.get_private_attributes()[0].to_bs58(),
            state.voucher.get_private_attributes()[1].to_bs58(),
            signature.to_bs58(),
            epoch_id.to_string(),
        )
        .await?;

    Ok(())
}

pub(crate) async fn recover_credentials<C: Clone + CosmWasmClient + Send + Sync>(
    client: validator_client::Client<C>,
    recovery_storage: &RecoveryStorage,
    shared_storage: PersistentStorage,
) -> Result<()> {
    for voucher in recovery_storage.unconsumed_vouchers()? {
        let state = State {
            amount: 0,
            voucher,
            params: Parameters::new(TOTAL_ATTRIBUTES).unwrap(),
        };
        if let Err(e) = get_credential(&state, client.clone(), shared_storage.clone()).await {
            println!(
                "Could not recover deposit {} due to {:?}, try again later",
                state.voucher.tx_hash(),
                e
            )
        } else {
            println!(
                "Converted deposit {} to a credential, removing recovery data for it",
                state.voucher.tx_hash()
            );
            if let Err(e) = recovery_storage.remove_voucher(state.voucher.tx_hash().to_string()) {
                println!("Could not remove recovery data - {:?}", e);
            }
        }
    }

    Ok(())
}
