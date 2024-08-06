// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{BasicTicketbookInformation, RetrievedPendingTicketbook, RetrievedTicketbook};
use async_trait::async_trait;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
use nym_ecash_time::Date;
use std::error::Error;

// for future reference, if you want to make a query for "how much bandwidth do we have left"
// do something along the lines of
// `SELECT total_tickets, used_tickets FROM ecash_ticketbook WHERE expiration_date >= ?`, today_date
// then for each calculate the diff total_tickets - used_tickets and multiply the result by the size of the ticket
#[async_trait]
pub trait Storage: Send + Sync {
    type StorageError: Error;

    /// remove all expired ticketbooks and expiration date signatures
    async fn cleanup_expired(&self) -> Result<(), Self::StorageError>;

    async fn insert_pending_ticketbook(
        &self,
        ticketbook: &IssuanceTicketBook,
    ) -> Result<(), Self::StorageError>;

    async fn insert_issued_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
    ) -> Result<(), Self::StorageError>;

    async fn get_ticketbooks_info(
        &self,
    ) -> Result<Vec<BasicTicketbookInformation>, Self::StorageError>;

    async fn get_pending_ticketbooks(
        &self,
    ) -> Result<Vec<RetrievedPendingTicketbook>, Self::StorageError>;

    async fn remove_pending_ticketbook(&self, pending_id: i64) -> Result<(), Self::StorageError>;

    /// Tries to retrieve one of the stored ticketbook,
    /// that has not yet expired and has required number of unspent tickets.
    /// it immediately updated the on-disk number of used tickets so that another task
    /// could obtain their own tickets at the same time
    async fn get_next_unspent_usable_ticketbook(
        &self,
        tickets: u32,
    ) -> Result<Option<RetrievedTicketbook>, Self::StorageError>;

    async fn attempt_revert_ticketbook_withdrawal(
        &self,
        ticketbook_id: i64,
        withdrawn: u32,
        expected_current_total_spent: u32,
    ) -> Result<bool, Self::StorageError>;

    async fn get_master_verification_key(
        &self,
        epoch_id: u64,
    ) -> Result<Option<VerificationKeyAuth>, Self::StorageError>;

    async fn insert_master_verification_key(
        &self,
        epoch_id: u64,
        key: &VerificationKeyAuth,
    ) -> Result<(), Self::StorageError>;

    async fn get_coin_index_signatures(
        &self,
        epoch_id: u64,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, Self::StorageError>;

    async fn insert_coin_index_signatures(
        &self,
        epoch_id: u64,
        data: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), Self::StorageError>;

    async fn get_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<Vec<AnnotatedExpirationDateSignature>>, Self::StorageError>;

    async fn insert_expiration_date_signatures(
        &self,
        epoch_id: u64,
        expiration_date: Date,
        data: &[AnnotatedExpirationDateSignature],
    ) -> Result<(), Self::StorageError>;
}
