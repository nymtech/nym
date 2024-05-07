// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_credential_storage::models::StorableIssuedCredential;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::{CredentialType, IssuanceBandwidthCredential};
use nym_credentials::coconut::utils::{
    obtain_aggregate_wallet, obtain_coin_indices_signatures, obtain_expiration_date_signatures,
    signatures_to_string,
};
use nym_credentials::obtain_aggregate_verification_key;
use nym_crypto::asymmetric::identity;
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::nyxd::contract_traits::EcashSigningClient;
use rand::rngs::OsRng;
use state::State;
use std::mem;
use zeroize::Zeroizing;

pub mod state;

pub async fn deposit<C>(client: &C, client_id: &[u8]) -> Result<State, BandwidthControllerError>
where
    C: EcashSigningClient + Sync,
{
    let mut rng = OsRng;
    let signing_key = identity::PrivateKey::new(&mut rng);

    let raw_deposit_id = client
        .deposit(
            CredentialType::TicketBook.to_string(),
            signing_key.public_key().to_base58_string(),
            None,
        )
        .await?
        .data;

    if raw_deposit_id.len() != mem::size_of::<DepositId>() {
        return Err(BandwidthControllerError::MalformedDepositId);
    }
    // SAFETY: we just checked for the correct length
    #[allow(clippy::unwrap_used)]
    let deposit_id = DepositId::from_be_bytes(raw_deposit_id.try_into().unwrap());

    let voucher = IssuanceBandwidthCredential::new_voucher(deposit_id, client_id, signing_key);

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
    assert!(state.voucher.typ().is_ticketbook());

    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let threshold = client
        .get_current_epoch_threshold()
        .await?
        .ok_or(BandwidthControllerError::NoThreshold)?;

    let ecash_api_clients = all_ecash_api_clients(client, epoch_id).await?;

    let verification_key = obtain_aggregate_verification_key(&ecash_api_clients)?;

    log::info!("Querying wallet signatures");
    let wallet = obtain_aggregate_wallet(&state.voucher, &ecash_api_clients, threshold).await?;

    log::info!("Querying expiration date signatures");
    let exp_date_sig =
        obtain_expiration_date_signatures(&ecash_api_clients, &verification_key, threshold).await?;

    log::info!("Checking coin indices signatures presence");
    if !storage
        .is_coin_indices_sig_present(
            epoch_id
                .try_into()
                .expect("our epoch id has run over i64::MAX!"),
        )
        .await
        .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?
    {
        log::info!("Querying coin indices signatures");
        let coin_indices_signatures =
            obtain_coin_indices_signatures(&ecash_api_clients, &verification_key, threshold)
                .await?;

        storage
            .insert_coin_indices_sig(
                epoch_id
                    .try_into()
                    .expect("our epoch id has run over i64::MAX!"),
                signatures_to_string(&coin_indices_signatures),
            )
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?;
    }

    let issued = state
        .voucher
        .to_issued_credential(wallet, exp_date_sig, epoch_id);

    // make sure the data gets zeroized after persisting it
    let credential_data = Zeroizing::new(issued.pack_v1());
    let storable = StorableIssuedCredential {
        serialization_revision: issued.current_serialization_revision(),
        credential_data: credential_data.as_ref(),
        credential_type: issued.typ().to_string(),
        epoch_id: epoch_id
            .try_into()
            .expect("our epoch id has run over u32::MAX!"),
    };

    storage
        .insert_issued_credential(storable)
        .await
        .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
}
