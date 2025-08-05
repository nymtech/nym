// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::helpers::IssuedExpirationDateSignatures;
use crate::ecash::storage::helpers::{
    deserialise_coin_index_signatures, deserialise_expiration_date_signatures,
    serialise_coin_index_signatures, serialise_expiration_date_signatures,
};
use crate::ecash::storage::manager::EcashStorageManagerExt;
use crate::ecash::storage::models::{
    IssuedHash, IssuedTicketbooksCount, IssuedTicketbooksForCount, IssuedTicketbooksOnCount,
    SerialNumberWrapper, TicketProvider,
};
use crate::node_status_api::models::NymApiStorageError;
use crate::support::storage::NymApiStorage;
use async_trait::async_trait;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::{BlindedSignature, VerificationKeyAuth};
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::TicketType;
use nym_ecash_contract_common::deposit::DepositId;
use nym_ticketbooks_merkle::{IssuedTicketbook, MerkleLeaf};
use nym_validator_client::nyxd::AccountId;
use std::collections::HashSet;
use time::{Date, OffsetDateTime};
use tracing::warn;

mod helpers;
pub(crate) mod manager;
pub(crate) mod models;

#[async_trait]
pub trait EcashStorageExt {
    async fn remove_expired_verified_tickets(&self, cutoff: Date)
        -> Result<(), NymApiStorageError>;

    async fn get_issued_partial_signature(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<BlindedSignature>, NymApiStorageError>;

    async fn get_issued_hashes(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedHash>, NymApiStorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn store_issued_ticketbook(
        &self,
        deposit_id: DepositId,
        dkg_epoch_id: u32,
        blinded_partial_credential: &[u8],
        joined_private_commitments: &[u8],
        expiration_date: Date,
        ticketbook_type: TicketType,
        merkle_leaf: MerkleLeaf,
    ) -> Result<(), NymApiStorageError>;

    async fn remove_old_issued_ticketbooks(
        &self,
        cutoff_expiration_date: Date,
    ) -> Result<(), NymApiStorageError>;

    async fn get_issued_ticketbooks(
        &self,
        deposits: &[DepositId],
    ) -> Result<Vec<IssuedTicketbook>, NymApiStorageError>;

    async fn get_credential_data(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<CredentialSpendingData>, NymApiStorageError>;

    /// Returns a boolean to indicate whether the ticket has actually been inserted
    async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<bool, NymApiStorageError>;

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, NymApiStorageError>;

    async fn get_verified_tickets_since(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError>;

    async fn update_last_batch_verification(
        &self,
        provider_id: i64,
        last_batch_verification: OffsetDateTime,
    ) -> Result<(), NymApiStorageError>;

    #[allow(dead_code)]
    async fn get_all_spent_tickets_on(
        &self,
        date: Date,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError>;

    async fn get_or_create_ticket_provider_with_id(
        &self,
        gateway_address: &str,
    ) -> Result<i64, NymApiStorageError>;

    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<VerificationKeyAuth>, NymApiStorageError>;

    async fn insert_master_verification_key(
        &self,
        epoch_id: EpochId,
        key: &VerificationKeyAuth,
    ) -> Result<(), NymApiStorageError>;

    async fn get_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError>;

    async fn insert_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError>;

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError>;

    async fn insert_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError>;

    async fn get_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError>;

    async fn insert_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError>;

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError>;

    async fn insert_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError>;

    async fn get_issued_ticketbooks_count(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<IssuedTicketbooksCount>, NymApiStorageError>;

    async fn get_issued_ticketbooks_on_count(
        &self,
        issuance_date: Date,
    ) -> Result<Vec<IssuedTicketbooksOnCount>, NymApiStorageError>;

    async fn get_issued_ticketbooks_for_count(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedTicketbooksForCount>, NymApiStorageError>;
}

#[async_trait]
impl EcashStorageExt for NymApiStorage {
    async fn remove_expired_verified_tickets(
        &self,
        cutoff: Date,
    ) -> Result<(), NymApiStorageError> {
        Ok(self.manager.remove_expired_verified_tickets(cutoff).await?)
    }

    async fn get_issued_partial_signature(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<BlindedSignature>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_issued_partial_signature(deposit_id)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(BlindedSignature::from_bytes(&raw).map_err(|err| {
            NymApiStorageError::database_inconsistency(format!(
                "failed to recover stored partial signature: {err}"
            ))
        })?))
    }

    async fn get_issued_hashes(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedHash>, NymApiStorageError> {
        Ok(self.manager.get_issued_hashes(expiration_date).await?)
    }

    #[allow(clippy::too_many_arguments)]
    async fn store_issued_ticketbook(
        &self,
        deposit_id: DepositId,
        dkg_epoch_id: u32,
        blinded_partial_credential: &[u8],
        joined_private_commitments: &[u8],
        expiration_date: Date,
        ticketbook_type: TicketType,
        merkle_leaf: MerkleLeaf,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .store_issued_ticketbook(
                deposit_id,
                dkg_epoch_id,
                blinded_partial_credential,
                joined_private_commitments,
                expiration_date,
                ticketbook_type.encode(),
                &merkle_leaf.hash,
                merkle_leaf.index as u32,
            )
            .await?)
    }

    async fn remove_old_issued_ticketbooks(
        &self,
        cutoff_expiration_date: Date,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .remove_old_issued_ticketbooks(cutoff_expiration_date)
            .await?)
    }

    async fn get_issued_ticketbooks(
        &self,
        deposits: &[DepositId],
    ) -> Result<Vec<IssuedTicketbook>, NymApiStorageError> {
        let raw = self.manager.get_issued_ticketbooks(deposits).await?;
        if raw.len() != deposits.len() {
            warn!("failed to get ticketbooks for all requested deposits. requested {} but only got {}", raw.len(), deposits.len());
            let available: HashSet<_> = raw.iter().map(|t| t.deposit_id).collect();
            let mut missing = Vec::new();
            for &requested in deposits {
                if !available.contains(&requested) {
                    warn!("the storage is missing ticketbook for deposit {requested}");
                    missing.push(requested);
                }
            }
            return Err(NymApiStorageError::UnavailableTicketbooks { deposits: missing });
        }
        raw.into_iter().map(TryInto::try_into).collect()
    }

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

    /// Returns a boolean to indicate whether the ticket has actually been inserted
    async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<bool, NymApiStorageError> {
        let provider_id = self
            .get_or_create_ticket_provider_with_id(gateway_addr.as_ref())
            .await?;

        let now = OffsetDateTime::now_utc();

        let ticket_bytes = ticket_data.to_bytes();
        let encoded_serial_number = ticket_data.encoded_serial_number();
        self.manager
            .insert_verified_ticket(
                provider_id,
                ticket_data.spend_date,
                now,
                ticket_bytes,
                encoded_serial_number,
            )
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

    async fn update_last_batch_verification(
        &self,
        provider_id: i64,
        last_batch_verification: OffsetDateTime,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .update_last_batch_verification(provider_id, last_batch_verification)
            .await?)
    }

    #[allow(dead_code)]
    async fn get_all_spent_tickets_on(
        &self,
        date: Date,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError> {
        self.manager
            .get_spent_tickets_on(date)
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

    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<VerificationKeyAuth>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_master_verification_key(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let master_vk = VerificationKeyAuth::from_bytes(&raw).map_err(|_| {
            NymApiStorageError::database_inconsistency("malformed stored master verification key")
        })?;

        Ok(Some(master_vk))
    }

    async fn insert_master_verification_key(
        &self,
        epoch_id: EpochId,
        key: &VerificationKeyAuth,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .insert_master_verification_key(epoch_id as i64, &key.to_bytes())
            .await?)
    }

    async fn get_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_partial_coin_index_signatures(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(deserialise_coin_index_signatures(&raw)?))
    }

    async fn insert_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_partial_coin_index_signatures(
                epoch_id as i64,
                &serialise_coin_index_signatures(sigs),
            )
            .await?;
        Ok(())
    }

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_master_coin_index_signatures(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(deserialise_coin_index_signatures(&raw)?))
    }

    async fn insert_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_master_coin_index_signatures(
                epoch_id as i64,
                &serialise_coin_index_signatures(sigs),
            )
            .await?;
        Ok(())
    }

    async fn get_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_partial_expiration_date_signatures(expiration_date, epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let signatures = deserialise_expiration_date_signatures(&raw.serialised_signatures)?;

        Ok(Some(IssuedExpirationDateSignatures {
            epoch_id: raw.epoch_id as u64,
            signatures,
        }))
    }

    async fn insert_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_partial_expiration_date_signatures(
                sigs.epoch_id as i64,
                expiration_date,
                &serialise_expiration_date_signatures(&sigs.signatures),
            )
            .await?;
        Ok(())
    }

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_master_expiration_date_signatures(expiration_date, epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let signatures = deserialise_expiration_date_signatures(&raw.serialised_signatures)?;

        Ok(Some(IssuedExpirationDateSignatures {
            epoch_id: raw.epoch_id as u64,
            signatures,
        }))
    }

    async fn insert_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_master_expiration_date_signatures(
                sigs.epoch_id as i64,
                expiration_date,
                &serialise_expiration_date_signatures(&sigs.signatures),
            )
            .await?;
        Ok(())
    }

    async fn get_issued_ticketbooks_count(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<IssuedTicketbooksCount>, NymApiStorageError> {
        Ok(self
            .manager
            .get_issued_ticketbooks_count(limit, offset)
            .await?)
    }

    async fn get_issued_ticketbooks_on_count(
        &self,
        issuance_date: Date,
    ) -> Result<Vec<IssuedTicketbooksOnCount>, NymApiStorageError> {
        Ok(self
            .manager
            .get_issued_ticketbooks_on_count(issuance_date)
            .await?)
    }

    async fn get_issued_ticketbooks_for_count(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedTicketbooksForCount>, NymApiStorageError> {
        Ok(self
            .manager
            .get_issued_ticketbooks_for_count(expiration_date)
            .await?)
    }
}
