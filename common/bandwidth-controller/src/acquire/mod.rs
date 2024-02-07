// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_coconut::Base58;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::{CredentialType, IssuanceBandwidthCredential};
use nym_credentials::coconut::utils::obtain_aggregate_signature;
use nym_crypto::asymmetric::{encryption, identity};
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nyxd::contract_traits::CoconutBandwidthSigningClient;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::nyxd::Coin;
use rand::rngs::OsRng;
use state::State;

pub mod state;

pub async fn deposit<C>(client: &C, amount: Coin) -> Result<State, BandwidthControllerError>
where
    C: CoconutBandwidthSigningClient + Sync,
{
    let mut rng = OsRng;
    let signing_key = identity::PrivateKey::new(&mut rng);
    let encryption_key = encryption::PrivateKey::new(&mut rng);

    let tx_hash = client
        .deposit(
            amount.clone(),
            CredentialType::Voucher.to_string(),
            signing_key.public_key().to_base58_string(),
            encryption_key.public_key().to_base58_string(),
            None,
        )
        .await?
        .transaction_hash;

    let voucher =
        IssuanceBandwidthCredential::new_voucher(amount, tx_hash, signing_key, encryption_key);

    let state = State { voucher };

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
    // temporary
    assert!(!state.voucher.typ().is_free_pass());

    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let threshold = client
        .get_current_epoch_threshold()
        .await?
        .ok_or(BandwidthControllerError::NoThreshold)?;

    let coconut_api_clients = all_coconut_api_clients(client, epoch_id).await?;

    let signature =
        obtain_aggregate_signature(&state.voucher, &coconut_api_clients, threshold).await?;

    // we asserted the that the bandwidth credential we obtained is **NOT** the free pass
    // so the first public attribute must be the value
    let voucher_value = state.voucher.get_plain_public_attributes()[0].clone();
    storage
        .insert_coconut_credential(
            voucher_value,
            CredentialType::Voucher.to_string(),
            state.voucher.get_private_attributes()[0].to_bs58(),
            state.voucher.get_private_attributes()[1].to_bs58(),
            signature.to_bs58(),
            epoch_id.to_string(),
        )
        .await
        .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
}
