// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_coconut_interface::Base58;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use nym_credentials::coconut::utils::obtain_aggregate_signature;
use nym_crypto::asymmetric::{encryption, identity};
use nym_network_defaults::VOUCHER_INFO;
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
    let params = BandwidthVoucher::default_parameters();
    let voucher_value = amount.amount.to_string();

    let tx_hash = client
        .deposit(
            amount,
            String::from(VOUCHER_INFO),
            signing_key.public_key().to_base58_string(),
            encryption_key.public_key().to_base58_string(),
            None,
        )
        .await?
        .transaction_hash;

    let voucher = BandwidthVoucher::new(
        &params,
        voucher_value,
        VOUCHER_INFO.to_string(),
        tx_hash,
        signing_key,
        encryption_key,
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
