// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_credential_storage::models::StorableIssuedCredential;
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
use zeroize::Zeroizing;

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

pub async fn get_bandwidth_voucher<C, St>(
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
    assert!(state.voucher.typ().is_voucher());

    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let threshold = client
        .get_current_epoch_threshold()
        .await?
        .ok_or(BandwidthControllerError::NoThreshold)?;

    let coconut_api_clients = all_coconut_api_clients(client, epoch_id).await?;

    let signature =
        obtain_aggregate_signature(&state.voucher, &coconut_api_clients, threshold).await?;
    let issued = state.voucher.to_issued_credential(signature);

    // make sure the data gets zeroized after persisting it
    let credential_data = Zeroizing::new(issued.pack_v1());
    let storable = StorableIssuedCredential {
        serialization_revision: issued.current_serialization_revision(),
        credential_data: credential_data.as_ref(),
        credential_type: issued.typ().to_string(),
        epoch_id: epoch_id
            .try_into()
            .expect("our epoch is has run over u32::MAX!"),
    };

    storage
        .insert_issued_credential(storable)
        .await
        .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
}
