// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_coconut_interface::{Base58, Parameters};
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::{BandwidthVoucher, TOTAL_ATTRIBUTES};
use nym_credentials::coconut::utils::obtain_aggregate_signature;
use nym_crypto::asymmetric::{encryption, identity};
use nym_network_defaults::VOUCHER_INFO;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nyxd::contract_traits::CoconutBandwidthSigningClient;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::Hash;
use rand::rngs::OsRng;
use state::{KeyPair, State};
use std::str::FromStr;

pub mod state;

pub async fn deposit<C>(
    client: &C,
    amount: impl Into<Coin>,
) -> Result<State, BandwidthControllerError>
where
    C: CoconutBandwidthSigningClient + Sync,
{
    let mut rng = OsRng;
    let signing_keypair = KeyPair::from(identity::KeyPair::new(&mut rng));
    let encryption_keypair = KeyPair::from(encryption::KeyPair::new(&mut rng));
    let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();
    let amount = amount.into();
    let voucher_value = amount.amount.to_string();

    let tx_hash = client
        .deposit(
            amount,
            String::from(VOUCHER_INFO),
            signing_keypair.public_key.clone(),
            encryption_keypair.public_key.clone(),
            None,
        )
        .await?
        .transaction_hash
        .to_string();

    let voucher = BandwidthVoucher::new(
        &params,
        voucher_value,
        VOUCHER_INFO.to_string(),
        Hash::from_str(&tx_hash).map_err(|_| BandwidthControllerError::InvalidTxHash)?,
        identity::PrivateKey::from_base58_string(&signing_keypair.private_key)?,
        encryption::PrivateKey::from_base58_string(&encryption_keypair.private_key)?,
    );

    let state = State { voucher, params };

    Ok(state)
}

pub async fn get_credential<C, St>(
    state: &State,
    client: &C,
    storage: &St,
) -> Result<(), BandwidthControllerError>
where
    C: DkgQueryClient + Send + Sync,
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let threshold = client
        .get_current_epoch_threshold()
        .await?
        .ok_or(BandwidthControllerError::NoThreshold)?;

    let coconut_api_clients = all_coconut_api_clients(client, epoch_id).await?;

    let signature = obtain_aggregate_signature(
        &state.params,
        &state.voucher,
        &coconut_api_clients,
        threshold,
    )
    .await?;
    storage
        .insert_coconut_credential(
            state.voucher.get_voucher_value(),
            VOUCHER_INFO.to_string(),
            state.voucher.get_private_attributes()[0].to_bs58(),
            state.voucher.get_private_attributes()[1].to_bs58(),
            signature.to_bs58(),
            epoch_id.to_string(),
        )
        .await
        .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
}
