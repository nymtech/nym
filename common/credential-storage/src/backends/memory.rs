// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{BasicTicketbookInformation, RetrievedPendingTicketbook, RetrievedTicketbook};
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
use nym_ecash_time::Date;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct MemoryEcachTicketbookManager {
    inner: Arc<RwLock<EcashCredentialManagerInner>>,
}

#[derive(Default)]
struct EcashCredentialManagerInner {
    ticketbooks: HashMap<i64, RetrievedTicketbook>,
    pending: HashMap<i64, RetrievedPendingTicketbook>,
    master_vk: HashMap<u64, VerificationKeyAuth>,
    coin_indices_sigs: HashMap<u64, Vec<AnnotatedCoinIndexSignature>>,
    expiration_date_sigs: HashMap<Date, Vec<AnnotatedExpirationDateSignature>>,
    _next_id: i64,
}

impl EcashCredentialManagerInner {
    fn next_id(&mut self) -> i64 {
        let next = self._next_id;
        self._next_id += 1;
        next
    }
}

// hehe, that's hacky AF, but it works as a **TEMPORARY** workaround
fn hack_clone_ticketbook(book: &IssuedTicketBook) -> IssuedTicketBook {
    let ser = book.pack();
    let data = Zeroizing::new(ser.data);
    IssuedTicketBook::try_unpack(&data, None).unwrap()
}

impl MemoryEcachTicketbookManager {
    /// Creates new empty instance of the `CoconutCredentialManager`.
    pub fn new() -> Self {
        MemoryEcachTicketbookManager {
            inner: Default::default(),
        }
    }

    pub(crate) async fn cleanup_expired(&self) {
        let mut guard = self.inner.write().await;

        let mut to_remove = Vec::new();

        for t in guard.ticketbooks.values() {
            if t.ticketbook.expired() {
                to_remove.push(t.ticketbook_id);
            }
        }

        for id in to_remove {
            guard.ticketbooks.remove(&id);
        }
    }

    pub async fn get_next_unspent_ticketbook_and_update(
        &self,
        ticketbook_type: String,
        tickets: u32,
    ) -> Option<RetrievedTicketbook> {
        let mut guard = self.inner.write().await;

        for t in guard.ticketbooks.values_mut() {
            if !t.ticketbook.expired()
                && t.ticketbook.spent_tickets() + tickets as u64
                    <= t.ticketbook.params_total_tickets()
                && t.ticketbook.ticketbook_type().to_string() == ticketbook_type
            {
                t.ticketbook
                    .update_spent_tickets(t.ticketbook.spent_tickets() + tickets as u64);
                return Some(RetrievedTicketbook {
                    ticketbook_id: t.ticketbook_id,
                    ticketbook: hack_clone_ticketbook(&t.ticketbook),
                });
            }
        }

        None
    }

    pub(crate) async fn revert_ticketbook_withdrawal(
        &self,
        ticketbook_id: i64,
        withdrawn: u32,
        expected_current_total_spent: u32,
    ) -> bool {
        let mut guard = self.inner.write().await;

        let Some(book) = guard.ticketbooks.get_mut(&ticketbook_id) else {
            return false;
        };

        if book.ticketbook.spent_tickets() == expected_current_total_spent as u64 {
            book.ticketbook
                .update_spent_tickets(book.ticketbook.spent_tickets() - withdrawn as u64);
            true
        } else {
            false
        }
    }

    pub(crate) async fn insert_pending_ticketbook(&self, ticketbook: &IssuanceTicketBook) {
        let mut guard = self.inner.write().await;

        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        let id = ticketbook.deposit_id() as i64;
        guard.pending.insert(
            id,
            RetrievedPendingTicketbook {
                pending_id: ticketbook.deposit_id() as i64,
                pending_ticketbook: IssuanceTicketBook::try_unpack(&data, None).unwrap(),
            },
        );
    }

    pub(crate) async fn get_pending_ticketbooks(&self) -> Vec<RetrievedPendingTicketbook> {
        let guard = self.inner.read().await;

        let mut pending = Vec::new();

        for p in guard.pending.values() {
            // 🫠
            let ser = p.pending_ticketbook.pack();
            let data = Zeroizing::new(ser.data);
            pending.push(RetrievedPendingTicketbook {
                pending_id: p.pending_id,
                pending_ticketbook: IssuanceTicketBook::try_unpack(&data, None).unwrap(),
            })
        }

        pending
    }

    pub(crate) async fn remove_pending_ticketbook(&self, pending_id: i64) {
        let mut guard = self.inner.write().await;

        guard.pending.remove(&pending_id);
    }

    pub(crate) async fn insert_new_ticketbook(&self, ticketbook: &IssuedTicketBook) {
        let mut guard = self.inner.write().await;
        let id = guard.next_id();

        // hehe, that's hacky AF, but it works as a **TEMPORARY** workaround
        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        guard.ticketbooks.insert(
            id,
            RetrievedTicketbook {
                ticketbook_id: id,
                ticketbook: IssuedTicketBook::try_unpack(&data, None).unwrap(),
            },
        );
    }

    pub(crate) async fn get_ticketbooks_info(&self) -> Vec<BasicTicketbookInformation> {
        let guard = self.inner.read().await;

        guard
            .ticketbooks
            .values()
            .map(|t| BasicTicketbookInformation {
                id: t.ticketbook_id,
                expiration_date: t.ticketbook.expiration_date(),
                ticketbook_type: t.ticketbook.ticketbook_type().to_string(),
                epoch_id: t.ticketbook.epoch_id() as u32,
                total_tickets: t.ticketbook.spent_tickets() as u32,
                used_tickets: t.ticketbook.params_total_tickets() as u32,
            })
            .collect()
    }

    pub(crate) async fn get_master_verification_key(
        &self,
        epoch_id: u64,
    ) -> Option<VerificationKeyAuth> {
        let guard = self.inner.read().await;

        guard.master_vk.get(&epoch_id).cloned()
    }

    pub(crate) async fn insert_master_verification_key(&self, key: &EpochVerificationKey) {
        let mut guard = self.inner.write().await;

        guard.master_vk.insert(key.epoch_id, key.key.clone());
    }

    pub(crate) async fn get_coin_index_signatures(
        &self,
        epoch_id: u64,
    ) -> Option<Vec<AnnotatedCoinIndexSignature>> {
        let guard = self.inner.read().await;

        guard.coin_indices_sigs.get(&epoch_id).cloned()
    }

    pub(crate) async fn insert_coin_index_signatures(
        &self,
        sigs: &AggregatedCoinIndicesSignatures,
    ) {
        let mut guard = self.inner.write().await;

        guard
            .coin_indices_sigs
            .insert(sigs.epoch_id, sigs.signatures.clone());
    }

    pub(crate) async fn get_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Option<Vec<AnnotatedExpirationDateSignature>> {
        let guard = self.inner.read().await;

        guard.expiration_date_sigs.get(&expiration_date).cloned()
    }

    pub(crate) async fn insert_expiration_date_signatures(
        &self,
        sigs: &AggregatedExpirationDateSignatures,
    ) {
        let mut guard = self.inner.write().await;

        guard
            .expiration_date_sigs
            .insert(sigs.expiration_date, sigs.signatures.clone());
    }
}
