// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::storage::manager::CoconutStorageManagerExt;
use crate::coconut::storage::models::{join_attributes, EpochCredentials, IssuedCredential};
use crate::node_status_api::models::NymApiStorageError;
use crate::support::storage::NymApiStorage;
use nym_coconut::{Base58, BlindedSignature};
use nym_coconut_dkg_common::types::EpochId;
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::Hash;

pub(crate) mod manager;
pub(crate) mod models;

#[async_trait]
pub trait CoconutStorageExt {
    #[deprecated]
    async fn get_blinded_signature_response(
        &self,
        tx_hash: &str,
    ) -> Result<Option<String>, NymApiStorageError> {
        Ok(None)
    }

    #[deprecated]
    async fn insert_blinded_signature_response(
        &self,
        tx_hash: &str,
        blinded_signature_response: &str,
    ) -> Result<(), NymApiStorageError> {
        Ok(())
    }

    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, NymApiStorageError>;

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
        public_attributes: Vec<String>,
    ) -> Result<i64, NymApiStorageError>;
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
        public_attributes: Vec<String>,
    ) -> Result<i64, NymApiStorageError> {
        Ok(self
            .manager
            .store_issued_credential(
                epoch_id,
                tx_hash.to_string(),
                partial_credential.to_bs58(),
                signature.to_base58_string(),
                join_attributes(private_commitments),
                join_attributes(public_attributes),
            )
            .await?)
    }
}
