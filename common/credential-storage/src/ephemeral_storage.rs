// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backends::memory::MemoryEcachTicketbookManager;
use crate::error::StorageError;
use crate::models::{BasicTicketbookInformation, RetrievedPendingTicketbook, RetrievedTicketbook};
use crate::storage::Storage;
use async_trait::async_trait;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
use nym_ecash_time::Date;
use std::fmt::{self, Debug, Formatter};

pub type EphemeralCredentialStorage = EphemeralStorage;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct EphemeralStorage {
    storage_manager: MemoryEcachTicketbookManager,
}

impl Default for EphemeralStorage {
    fn default() -> Self {
        EphemeralStorage {
            storage_manager: MemoryEcachTicketbookManager::new(),
        }
    }
}

impl Debug for EphemeralStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "EphemeralStorage")
    }
}

#[async_trait]
impl Storage for EphemeralStorage {
    type StorageError = StorageError;

    async fn cleanup_expired(&self) -> Result<(), Self::StorageError> {
        self.storage_manager.cleanup_expired().await;
        Ok(())
    }

    async fn insert_pending_ticketbook(
        &self,
        ticketbook: &IssuanceTicketBook,
    ) -> Result<(), Self::StorageError> {
        self.storage_manager
            .insert_pending_ticketbook(ticketbook)
            .await;
        Ok(())
    }

    async fn insert_issued_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
    ) -> Result<(), StorageError> {
        self.storage_manager.insert_new_ticketbook(ticketbook).await;
        Ok(())
    }

    async fn get_ticketbooks_info(
        &self,
    ) -> Result<Vec<BasicTicketbookInformation>, Self::StorageError> {
        Ok(self.storage_manager.get_ticketbooks_info().await)
    }

    async fn get_pending_ticketbooks(
        &self,
    ) -> Result<Vec<RetrievedPendingTicketbook>, Self::StorageError> {
        Ok(self.storage_manager.get_pending_ticketbooks().await)
    }

    async fn remove_pending_ticketbook(&self, pending_id: i64) -> Result<(), Self::StorageError> {
        self.storage_manager
            .remove_pending_ticketbook(pending_id)
            .await;
        Ok(())
    }

    /// Tries to retrieve one of the stored ticketbook,
    /// that has not yet expired and has required number of unspent tickets.
    /// it immediately updated the on-disk number of used tickets so that another task
    /// could obtain their own tickets at the same time
    async fn get_next_unspent_usable_ticketbook(
        &self,
        tickets: u32,
    ) -> Result<Option<RetrievedTicketbook>, Self::StorageError> {
        Ok(self
            .storage_manager
            .get_next_unspent_ticketbook_and_update(tickets)
            .await)
    }

    async fn attempt_revert_ticketbook_withdrawal(
        &self,
        ticketbook_id: i64,
        previous_total_spent: u32,
        withdrawn: u32,
    ) -> Result<bool, Self::StorageError> {
        Ok(self
            .storage_manager
            .revert_ticketbook_withdrawal(ticketbook_id, previous_total_spent, withdrawn)
            .await)
    }

    async fn get_master_verification_key(
        &self,
        epoch_id: u64,
    ) -> Result<Option<VerificationKeyAuth>, Self::StorageError> {
        Ok(self
            .storage_manager
            .get_master_verification_key(epoch_id)
            .await)
    }

    async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), Self::StorageError> {
        self.storage_manager
            .insert_master_verification_key(key)
            .await;
        Ok(())
    }

    async fn get_coin_index_signatures(
        &self,
        epoch_id: u64,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, Self::StorageError> {
        Ok(self
            .storage_manager
            .get_coin_index_signatures(epoch_id)
            .await)
    }

    async fn insert_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), Self::StorageError> {
        self.storage_manager
            .insert_coin_index_signatures(signatures)
            .await;
        Ok(())
    }

    async fn get_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<Vec<AnnotatedExpirationDateSignature>>, Self::StorageError> {
        Ok(self
            .storage_manager
            .get_expiration_date_signatures(expiration_date)
            .await)
    }

    async fn insert_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), Self::StorageError> {
        self.storage_manager
            .insert_expiration_date_signatures(signatures)
            .await;
        Ok(())
    }
}
