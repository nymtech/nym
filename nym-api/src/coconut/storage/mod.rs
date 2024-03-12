// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::storage::manager::CoconutStorageManagerExt;
use crate::coconut::storage::models::{join_attributes, EpochCredentials, IssuedCredential};
use crate::node_status_api::models::NymApiStorageError;
use crate::support::storage::NymApiStorage;
use nym_api_requests::coconut::models::Pagination;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::utils::BlindedSignature;
use nym_compact_ecash::Base58;
use nym_credentials::CredentialSpendingData;
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::{AccountId, Hash};

pub(crate) mod manager;
pub(crate) mod models;

const DEFAULT_CREDENTIALS_PAGE_LIMIT: u32 = 100;

#[async_trait]
pub trait CoconutStorageExt {
    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, NymApiStorageError>;

    #[allow(dead_code)]
    async fn create_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
    ) -> Result<(), NymApiStorageError>;

    async fn update_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
        credential_id: i64,
    ) -> Result<(), NymApiStorageError>;

    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedCredential>, NymApiStorageError>;

    async fn get_issued_bandwidth_credential_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Option<IssuedCredential>, NymApiStorageError>;

    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        tx_hash: Hash,
        partial_credential: &BlindedSignature,
        signature: identity::Signature,
        private_commitments: Vec<String>,
        expiration_date: i64,
    ) -> Result<i64, NymApiStorageError>;

    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedCredential>, NymApiStorageError>;

    async fn get_issued_credentials_paged(
        &self,
        pagination: Pagination<i64>,
    ) -> Result<Vec<IssuedCredential>, NymApiStorageError>;

    async fn insert_credential(
        &self,
        credential: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<(), NymApiStorageError>;

    async fn increment_issued_freepasses(&self) -> Result<(), NymApiStorageError>;
}

#[async_trait]
impl CoconutStorageExt for NymApiStorage {
    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, NymApiStorageError> {
        Ok(self.manager.get_epoch_credentials(epoch_id).await?)
    }

    async fn create_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .create_epoch_credentials_entry(epoch_id)
            .await?)
    }

    async fn update_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
        credential_id: i64,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .update_epoch_credentials_entry(epoch_id, credential_id)
            .await?)
    }

    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedCredential>, NymApiStorageError> {
        Ok(self.manager.get_issued_credential(credential_id).await?)
    }

    async fn get_issued_bandwidth_credential_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Option<IssuedCredential>, NymApiStorageError> {
        Ok(self
            .manager
            .get_issued_bandwidth_credential_by_hash(tx_hash)
            .await?)
    }

    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        tx_hash: Hash,
        partial_credential: &BlindedSignature,
        signature: identity::Signature,
        private_commitments: Vec<String>,
        expiration_date: i64,
    ) -> Result<i64, NymApiStorageError> {
        Ok(self
            .manager
            .store_issued_credential(
                epoch_id,
                tx_hash.to_string(),
                partial_credential.to_bs58(),
                signature.to_base58_string(),
                join_attributes(private_commitments),
                expiration_date,
            )
            .await?)
    }

    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedCredential>, NymApiStorageError> {
        Ok(self.manager.get_issued_credentials(credential_ids).await?)
    }

    async fn get_issued_credentials_paged(
        &self,
        pagination: Pagination<i64>,
    ) -> Result<Vec<IssuedCredential>, NymApiStorageError> {
        // rows start at 1
        let start_after = pagination.last_key.unwrap_or(0);
        let limit = match pagination.limit {
            Some(v) => {
                if v == 0 || v > DEFAULT_CREDENTIALS_PAGE_LIMIT {
                    DEFAULT_CREDENTIALS_PAGE_LIMIT
                } else {
                    v
                }
            }
            None => DEFAULT_CREDENTIALS_PAGE_LIMIT,
        };

        Ok(self
            .manager
            .get_issued_credentials_paged(start_after, limit)
            .await?)
    }

    async fn insert_credential(
        &self,
        credential: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_credential(credential.to_bs58(), gateway_addr.to_string())
            .await
            .map_err(|err| err.into())
    }

    async fn increment_issued_freepasses(&self) -> Result<(), NymApiStorageError> {
        Ok(self.manager.increment_issued_freepasses().await?)
    }
}
