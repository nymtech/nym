// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::storage::manager::CoconutStorageManagerExt;
use crate::ecash::storage::models::{
    join_attributes, EpochCredentials, IssuedCredential, SerialNumberWrapper, TicketProvider,
};
use crate::node_status_api::models::NymApiStorageError;
use crate::support::storage::NymApiStorage;
use nym_api_requests::coconut::models::Pagination;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::Base58;
use nym_compact_ecash::BlindedSignature;
use nym_credentials::CredentialSpendingData;
use nym_crypto::asymmetric::identity;
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::nyxd::AccountId;
use time::OffsetDateTime;

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

    async fn get_issued_bandwidth_credential_by_deposit_id(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<IssuedCredential>, NymApiStorageError>;

    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        deposit_id: DepositId,
        partial_credential: &BlindedSignature,
        signature: identity::Signature,
        private_commitments: Vec<String>,
        expiration_date: OffsetDateTime,
    ) -> Result<i64, NymApiStorageError>;

    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedCredential>, NymApiStorageError>;

    async fn get_issued_credentials_paged(
        &self,
        pagination: Pagination<i64>,
    ) -> Result<Vec<IssuedCredential>, NymApiStorageError>;
    //
    // async fn insert_credential(
    //     &self,
    //     credential: &CredentialSpendingData,
    //     serial_number_bs58: String,
    //     gateway_addr: &AccountId,
    //     proposal_id: u64,
    // ) -> Result<(), NymApiStorageError>;
    //
    async fn get_credential_data(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<CredentialSpendingData>, NymApiStorageError>;

    async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<(), NymApiStorageError>;

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, NymApiStorageError>;

    async fn get_verified_tickets_since(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError>;
    async fn get_or_create_ticket_provider_with_id(
        &self,
        gateway_address: &str,
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

    async fn get_issued_bandwidth_credential_by_deposit_id(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<IssuedCredential>, NymApiStorageError> {
        Ok(self
            .manager
            .get_issued_bandwidth_credential_by_deposit_id(deposit_id)
            .await?)
    }

    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        deposit_id: DepositId,
        partial_credential: &BlindedSignature,
        signature: identity::Signature,
        private_commitments: Vec<String>,
        expiration_date: OffsetDateTime,
    ) -> Result<i64, NymApiStorageError> {
        Ok(self
            .manager
            .store_issued_credential(
                epoch_id,
                deposit_id,
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

    // async fn insert_credential(
    //     &self,
    //     credential: &CredentialSpendingData,
    //     serial_number_bs58: String,
    //     gateway_addr: &AccountId,
    //     proposal_id: u64,
    // ) -> Result<(), NymApiStorageError> {
    //     self.manager
    //         .insert_credential(
    //             credential.to_bs58(),
    //             serial_number_bs58,
    //             gateway_addr.to_string(),
    //             proposal_id as i64,
    //         )
    //         .await
    //         .map_err(|err| err.into())
    // }

    async fn get_credential_data(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<CredentialSpendingData>, NymApiStorageError> {
        let ticket = self.manager.get_ticket(serial_number).await?;
        ticket
            .map(|ticket| {
                CredentialSpendingData::try_from_bytes(&ticket.ticket_data).map_err(|_| {
                    NymApiStorageError::DatabaseInconsistency {
                        reason: "impossible to deserialize verified ticket".to_string(),
                    }
                })
            })
            .transpose()
    }

    async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<(), NymApiStorageError> {
        let provider_id = self
            .get_or_create_ticket_provider_with_id(gateway_addr.as_ref())
            .await?;

        let now = OffsetDateTime::now_utc();

        let ticket_bytes = ticket_data.to_bytes();
        let encoded_serial_number = ticket_data.encoded_serial_number();
        self.manager
            .insert_verified_ticket(provider_id, now, ticket_bytes, encoded_serial_number)
            .await
            .map_err(Into::into)
    }

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, NymApiStorageError> {
        self.manager
            .get_ticket_provider(gateway_address)
            .await
            .map_err(Into::into)
    }

    async fn get_verified_tickets_since(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError> {
        self.manager
            .get_provider_ticket_serial_numbers(provider_id, since)
            .await
            .map_err(Into::into)
    }

    async fn get_or_create_ticket_provider_with_id(
        &self,
        gateway_address: &str,
    ) -> Result<i64, NymApiStorageError> {
        if let Some(provider) = self.get_ticket_provider(gateway_address).await? {
            Ok(provider.id)
        } else {
            Ok(self.manager.insert_ticket_provider(gateway_address).await?)
        }
    }
}
