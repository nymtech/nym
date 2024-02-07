// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use crate::utils::stored_credential_to_issued_bandwidth;
use log::{error, warn};
use nym_credential_storage::error::StorageError;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use nym_credentials::coconut::utils::obtain_aggregate_verification_key;
use nym_credentials_interface::VerificationKey;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use std::str::FromStr;

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

impl<C, St: Storage> BandwidthController<C, St> {
    pub fn new(storage: St, client: C) -> Self {
        BandwidthController { storage, client }
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
        Ok(obtain_aggregate_verification_key(&coconut_api_clients).await?)
    }

    pub async fn prepare_bandwidth_credential(
        &self,
    ) -> Result<PreparedCredential, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let retrieved_credential = self
            .storage
            .get_next_unspent_credential()
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?;

        let epoch_id = u64::from_str(&retrieved_credential.epoch_id)
            .map_err(|_| StorageError::InconsistentData)?;
        let credential_id = retrieved_credential.id;

        let issued_bandwidth = stored_credential_to_issued_bandwidth(retrieved_credential)?;

        let verification_key = match self.get_aggregate_verification_key(epoch_id).await {
            Ok(key) => key,
            Err(err) => {
                warn!("failed to obtain master verification key: {err}. Putting the credential back into the database");

                // TODO: ERROR RECOVERY:
                error!("unimplemented: putting the credential back into the database");
                return Err(err);
            }
        };

        let spend_request = issued_bandwidth.prepare_for_spending(&verification_key)?;

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
