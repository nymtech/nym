// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use crate::utils::stored_credential_to_issued_bandwidth;
use log::info;
use log::{error, warn};

use nym_credential_storage::models::StorableIssuedCredential;
use nym_credentials::coconut::utils::{obtain_coin_indices_signatures, signatures_to_string};
use nym_credentials_interface::{constants, NymPayInfo, VerificationKeyAuth};

use nym_credential_storage::storage::Storage;

use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use nym_credentials::coconut::utils::signatures_from_string;
use nym_credentials::obtain_aggregate_verification_key;
use nym_credentials::IssuedBandwidthCredential;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use zeroize::Zeroizing;

pub mod acquire;
pub mod error;
mod utils;

#[derive(Debug)]
pub struct BandwidthController<C, St> {
    storage: St,
    client: C,
}

pub struct PreparedCredential {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier
    /// could use correct verification key for validation.
    pub epoch_id: EpochId,

    /// The database id of the stored credential.
    pub credential_id: i64,

    ///the updated credential after the payment
    pub updated_credential: IssuedBandwidthCredential,
}

pub struct RetrievedCredential {
    pub credential: IssuedBandwidthCredential,
    pub credential_id: i64,
}

impl<C, St: Storage> BandwidthController<C, St> {
    pub fn new(storage: St, client: C) -> Self {
        BandwidthController { storage, client }
    }

    /// Tries to retrieve one of the stored, unused credentials that hasn't yet expired.
    /// It marks any retrieved intermediate credentials as expired.
    pub async fn get_next_usable_credential(
        &self,
    ) -> Result<RetrievedCredential, BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        loop {
            let Some(maybe_next) = self
                .storage
                .get_next_unspent_credential()
                .await
                .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?
            else {
                return Err(BandwidthControllerError::NoCredentialsAvailable);
            };
            let id = maybe_next.id;

            // try to deserialize it
            let valid_credential = match stored_credential_to_issued_bandwidth(maybe_next) {
                // check if it has already expired
                Ok(credential) => {
                    if credential.expired() {
                        warn!("the credential (id: {id}) has already expired! The expiration was set to {}", credential.expiration_date());
                        self.storage.mark_expired(id).await.map_err(|err| {
                            BandwidthControllerError::CredentialStorageError(Box::new(err))
                        })?;
                        continue;
                    }
                    credential
                }
                Err(err) => {
                    error!("failed to deserialize credential with id {id}: {err}. it may need to be manually removed from the storage");
                    return Err(err);
                }
            };
            return Ok(RetrievedCredential {
                credential: valid_credential,
                credential_id: id,
            });
        }
    }

    pub fn storage(&self) -> &St {
        &self.storage
    }

    async fn get_aggregate_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<VerificationKeyAuth, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let coconut_api_clients = all_ecash_api_clients(&self.client, epoch_id).await?;
        Ok(obtain_aggregate_verification_key(&coconut_api_clients)?)
    }

    pub async fn prepare_ecash_credential(
        &self,
        provider_pk: [u8; 32],
    ) -> Result<PreparedCredential, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let mut retrieved_credential = self.get_next_usable_credential().await?;

        let epoch_id = retrieved_credential.credential.epoch_id();
        let credential_id = retrieved_credential.credential_id;

        let verification_key = self.get_aggregate_verification_key(epoch_id).await?;

        let coin_indices_signatures_bs58 = self
            .storage
            .get_coin_indices_sig(
                epoch_id
                    .try_into()
                    .expect("our epoch id has run over i64::MAX!"),
            )
            .await
            .ok();

        let coin_indices_signatures = match coin_indices_signatures_bs58 {
            Some(epoch_signatures) => signatures_from_string(epoch_signatures.signatures)?,
            None => {
                info!("We're missing some signatures, let's query them now");
                //let's try to query them if we don't have them at that point
                let ecash_api_client = all_ecash_api_clients(&self.client, epoch_id).await?;
                let threshold = self
                    .client
                    .get_current_epoch_threshold()
                    .await?
                    .ok_or(BandwidthControllerError::NoThreshold)?;

                let coin_indices_signatures =
                    obtain_coin_indices_signatures(&ecash_api_client, &verification_key, threshold)
                        .await?;

                self.storage
                    .insert_coin_indices_sig(
                        epoch_id
                            .try_into()
                            .expect("our epoch id has run over i64::MAX!"),
                        signatures_to_string(&coin_indices_signatures),
                    )
                    .await
                    .map_err(|err| {
                        BandwidthControllerError::CredentialStorageError(Box::new(err))
                    })?;
                coin_indices_signatures
            }
        };

        let pay_info = NymPayInfo::generate(provider_pk);

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)

        let spend_request = retrieved_credential.credential.prepare_for_spending(
            &verification_key,
            pay_info.into(),
            &coin_indices_signatures,
        )?;
        Ok(PreparedCredential {
            data: spend_request,
            epoch_id,
            credential_id,
            updated_credential: retrieved_credential.credential,
        })
    }

    pub async fn update_ecash_wallet(
        &self,
        credential: IssuedBandwidthCredential,
        id: i64,
    ) -> Result<(), BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        // JS: shouldn't we send some contract/validator/gateway message here to actually, you know,
        // consume it?
        let consumed = credential.wallet().tickets_spent() >= constants::NB_TICKETS;

        // make sure the data gets zeroized after persisting it
        let credential_data = Zeroizing::new(credential.pack_v1());
        let storable = StorableIssuedCredential {
            serialization_revision: credential.current_serialization_revision(),
            credential_data: credential_data.as_ref(),
            credential_type: credential.typ().to_string(),
            epoch_id: credential
                .epoch_id()
                .try_into()
                .expect("our epoch is has run over u32::MAX!"),
        };

        self.storage
            .update_issued_credential(storable, id, consumed)
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
    }
}

impl<C, St> Clone for BandwidthController<C, St>
where
    C: Clone,
    St: Clone,
{
    fn clone(&self) -> Self {
        BandwidthController {
            storage: self.storage.clone(),
            client: self.client.clone(),
        }
    }
}
