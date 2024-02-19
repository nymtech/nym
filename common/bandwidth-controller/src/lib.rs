// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use crate::utils::stored_credential_to_issued_bandwidth;
use log::{debug, error, warn};
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::issued::BandwidthCredentialIssuedDataVariant;
use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use nym_credentials::coconut::utils::obtain_aggregate_verification_key;
use nym_credentials::IssuedBandwidthCredential;
use nym_credentials_interface::VerificationKey;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;

pub mod acquire;
pub mod error;
mod utils;

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
                Ok(credential) => match credential.variant_data() {
                    BandwidthCredentialIssuedDataVariant::Voucher(_) => {
                        debug!("credential {id} is a bandwidth voucher");
                        credential
                    }
                    BandwidthCredentialIssuedDataVariant::FreePass(freepass_info) => {
                        debug!("credential {id} is a free pass");
                        if freepass_info.expired() {
                            warn!("the free pass (id: {id}) has already expired! The expiration was set to {}", freepass_info.expiry_date());
                            self.storage.mark_expired(id).await.map_err(|err| {
                                BandwidthControllerError::CredentialStorageError(Box::new(err))
                            })?;
                            continue;
                        }
                        credential
                    }
                },
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
    ) -> Result<VerificationKey, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let coconut_api_clients = all_coconut_api_clients(&self.client, epoch_id).await?;
        Ok(obtain_aggregate_verification_key(&coconut_api_clients)?)
    }

    pub async fn prepare_bandwidth_credential(
        &self,
    ) -> Result<PreparedCredential, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let retrieved_credential = self.get_next_usable_credential().await?;

        let epoch_id = retrieved_credential.credential.epoch_id();
        let credential_id = retrieved_credential.credential_id;

        let verification_key = self.get_aggregate_verification_key(epoch_id).await?;

        let spend_request = retrieved_credential
            .credential
            .prepare_for_spending(&verification_key)?;

        Ok(PreparedCredential {
            data: spend_request,
            epoch_id,
            credential_id,
        })
    }

    pub async fn consume_credential(&self, id: i64) -> Result<(), BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        // JS: shouldn't we send some contract/validator/gateway message here to actually, you know,
        // consume it?
        self.storage
            .consume_coconut_credential(id)
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
