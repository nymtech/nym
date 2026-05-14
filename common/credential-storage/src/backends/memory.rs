// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{
    BasicTicketbookInformation, EmergencyCredential, EmergencyCredentialContent,
    RetrievedPendingTicketbook, RetrievedTicketbook,
};
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
struct InternalIdCounters {
    next_ticketbook_id: i64,
    next_emergency_credential_id: i64,
}

#[derive(Default)]
struct EcashCredentialManagerInner {
    ticketbooks: HashMap<i64, RetrievedTicketbook>,
    pending: HashMap<i64, RetrievedPendingTicketbook>,
    master_vk: HashMap<u64, VerificationKeyAuth>,
    coin_indices_sigs: HashMap<u64, Vec<AnnotatedCoinIndexSignature>>,
    expiration_date_sigs: HashMap<(u64, Date), Vec<AnnotatedExpirationDateSignature>>,
    emergency_credentials: HashMap<String, Vec<EmergencyCredential>>,

    // internal counters emulating assignment of an increasing id to new inserted database entries
    internal_counters: InternalIdCounters,
}

impl EcashCredentialManagerInner {
    fn next_ticketbook_id(&mut self) -> i64 {
        let next = self.internal_counters.next_ticketbook_id;
        self.internal_counters.next_ticketbook_id += 1;
        next
    }

    fn next_emergency_credential_id(&mut self) -> i64 {
        let next = self.internal_counters.next_emergency_credential_id;
        self.internal_counters.next_emergency_credential_id += 1;
        next
    }
}

// hehe, that's hacky AF, but it works as a **TEMPORARY** workaround
fn hack_clone_ticketbook(book: &IssuedTicketBook) -> IssuedTicketBook {
    let ser = book.pack();
    let data = Zeroizing::new(ser.data);
    #[allow(clippy::unwrap_used)]
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
            if t.ticketbook.expired() {
                continue;
            }
            if t.ticketbook.spent_tickets() + tickets as u64 > t.total_tickets as u64 {
                continue;
            }
            if t.ticketbook.ticketbook_type().to_string() != ticketbook_type {
                continue;
            }

            let cloned = hack_clone_ticketbook(&t.ticketbook);
            t.ticketbook
                .update_spent_tickets(t.ticketbook.spent_tickets() + tickets as u64);
            return Some(RetrievedTicketbook {
                ticketbook_id: t.ticketbook_id,
                total_tickets: t.total_tickets,
                ticketbook: cloned,
            });
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

    pub(crate) async fn insert_new_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
        total_tickets: u32,
        used_tickets: u32,
    ) {
        let mut guard = self.inner.write().await;
        let id = guard.next_ticketbook_id();

        #[allow(clippy::unwrap_used)]
        let mut nasty_clone = hack_clone_ticketbook(ticketbook);
        nasty_clone.update_spent_tickets(used_tickets as u64);

        guard.ticketbooks.insert(
            id,
            RetrievedTicketbook {
                ticketbook_id: id,
                total_tickets,
                ticketbook: nasty_clone,
            },
        );
    }

    pub(crate) async fn contains_ticketbook(&self, ticketbook: &IssuedTicketBook) -> bool {
        let ser = ticketbook.pack();
        let search_data = Zeroizing::new(ser.data);
        self.inner
            .read()
            .await
            .ticketbooks
            .iter()
            .any(|ticketbook| {
                let ser = ticketbook.1.ticketbook.pack();
                let data = Zeroizing::new(ser.data);
                search_data.eq(&data)
            })
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
                total_tickets: t.total_tickets,
                used_tickets: t.ticketbook.spent_tickets() as u32,
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
        epoch_id: u64,
    ) -> Option<Vec<AnnotatedExpirationDateSignature>> {
        let guard = self.inner.read().await;

        guard
            .expiration_date_sigs
            .get(&(epoch_id, expiration_date))
            .cloned()
    }

    pub(crate) async fn insert_expiration_date_signatures(
        &self,
        sigs: &AggregatedExpirationDateSignatures,
    ) {
        let mut guard = self.inner.write().await;

        guard.expiration_date_sigs.insert(
            (sigs.epoch_id, sigs.expiration_date),
            sigs.signatures.clone(),
        );
    }

    pub(crate) async fn get_emergency_credential(&self, typ: &str) -> Option<EmergencyCredential> {
        let guard = self.inner.read().await;

        guard.emergency_credentials.get(typ)?.first().cloned()
    }

    pub(crate) async fn insert_emergency_credential(
        &self,
        credential: &EmergencyCredentialContent,
    ) {
        let mut guard = self.inner.write().await;
        let id = guard.next_emergency_credential_id();

        guard
            .emergency_credentials
            .entry(credential.typ.clone())
            .or_default()
            .push(EmergencyCredential {
                id,
                data: credential.clone(),
            });
    }

    pub(crate) async fn remove_emergency_credential(&self, id: i64) {
        let mut guard = self.inner.write().await;

        guard.emergency_credentials.retain(|_, credentials| {
            credentials.retain(|c| c.id != id);
            !credentials.is_empty()
        })
    }

    pub(crate) async fn remove_emergency_credentials_of_type(&self, typ: &str) {
        let mut guard = self.inner.write().await;
        guard.emergency_credentials.remove(typ);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_compact_ecash::tests::helpers::generate_expiration_date_signatures;
    use nym_compact_ecash::{issue, ttp_keygen};
    use nym_credentials_interface::TicketType;
    use nym_crypto::asymmetric::ed25519;
    use nym_ecash_time::EcashTime;
    use nym_test_utils::helpers::deterministic_rng;

    fn mock_issuance(deposit_id: u32) -> IssuanceTicketBook {
        let identifier = "foomp";
        let mut rng = deterministic_rng();
        let key = ed25519::PrivateKey::new(&mut rng);
        let typ = TicketType::V1MixnetEntry;
        IssuanceTicketBook::new(deposit_id, identifier, key, typ)
    }

    fn mock_ticketbook() -> anyhow::Result<IssuedTicketBook> {
        let signing_keys = ttp_keygen(1, 1)?.remove(0);
        let issuance = mock_issuance(42);
        let expiration_date = issuance.expiration_date();

        let sig_req = issuance.prepare_for_signing();
        let _exp_date_sigs = generate_expiration_date_signatures(
            sig_req.expiration_date.ecash_unix_timestamp(),
            &[signing_keys.secret_key()],
            &[signing_keys.verification_key()],
            &signing_keys.verification_key(),
            &[1],
        )?;
        let blind_sig = issue(
            signing_keys.secret_key(),
            sig_req.ecash_pub_key,
            &sig_req.withdrawal_request,
            expiration_date.ecash_unix_timestamp(),
            issuance.ticketbook_type().encode(),
        )?;

        let partial_wallet =
            issuance.unblind_signature(&signing_keys.verification_key(), &sig_req, blind_sig, 1)?;

        let wallet = issuance.aggregate_signature_shares(
            &signing_keys.verification_key(),
            &[partial_wallet],
            sig_req,
        )?;

        Ok(issuance.into_issued_ticketbook(wallet, 1))
    }

    fn mock_verification_key() -> VerificationKeyAuth {
        ttp_keygen(1, 1).unwrap().remove(0).verification_key()
    }

    #[tokio::test]
    async fn get_ticketbooks_info_empty() {
        let manager = MemoryEcachTicketbookManager::new();
        let info = manager.get_ticketbooks_info().await;
        assert!(info.is_empty());
    }

    #[tokio::test]
    async fn get_ticketbooks_info_maps_inserted_ticketbook() -> anyhow::Result<()> {
        let manager = MemoryEcachTicketbookManager::new();
        let ticketbook = mock_ticketbook()?;
        let total_tickets = 100;
        let used_tickets = 25;

        manager
            .insert_new_ticketbook(&ticketbook, total_tickets, used_tickets)
            .await;

        let info = manager.get_ticketbooks_info().await;
        assert_eq!(info.len(), 1);
        let entry = &info[0];
        assert_eq!(entry.id, 0);
        assert_eq!(entry.expiration_date, ticketbook.expiration_date());
        assert_eq!(
            entry.ticketbook_type,
            ticketbook.ticketbook_type().to_string()
        );
        assert_eq!(entry.epoch_id, ticketbook.epoch_id() as u32);
        assert_eq!(entry.total_tickets, total_tickets);
        assert_eq!(entry.used_tickets, used_tickets);

        Ok(())
    }

    #[tokio::test]
    async fn contains_ticketbook_reflects_insertion() -> anyhow::Result<()> {
        let manager = MemoryEcachTicketbookManager::new();
        let ticketbook = mock_ticketbook()?;

        assert!(!manager.contains_ticketbook(&ticketbook).await);

        manager.insert_new_ticketbook(&ticketbook, 100, 0).await;

        assert!(manager.contains_ticketbook(&ticketbook).await);
        Ok(())
    }

    #[tokio::test]
    async fn insert_new_ticketbook_assigns_incrementing_ids() -> anyhow::Result<()> {
        let manager = MemoryEcachTicketbookManager::new();
        let ticketbook = mock_ticketbook()?;

        manager.insert_new_ticketbook(&ticketbook, 100, 0).await;
        manager.insert_new_ticketbook(&ticketbook, 100, 0).await;

        let mut ids: Vec<i64> = manager
            .get_ticketbooks_info()
            .await
            .into_iter()
            .map(|i| i.id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec![0, 1]);
        Ok(())
    }

    #[tokio::test]
    async fn get_next_unspent_ticketbook_updates_spent_and_exhausts() -> anyhow::Result<()> {
        let manager = MemoryEcachTicketbookManager::new();
        let ticketbook = mock_ticketbook()?;
        let typ = ticketbook.ticketbook_type().to_string();

        // total = 3, used = 0 — leaves 3 tickets available
        manager.insert_new_ticketbook(&ticketbook, 3, 0).await;

        let first = manager
            .get_next_unspent_ticketbook_and_update(typ.clone(), 2)
            .await;
        assert!(first.is_some());
        let first = first.unwrap();
        assert_eq!(first.total_tickets, 3);
        // returned ticketbook reflects state *before* the update
        assert_eq!(first.ticketbook.spent_tickets(), 0);

        // next withdrawal of 2 should be rejected (only 1 left)
        let second = manager
            .get_next_unspent_ticketbook_and_update(typ.clone(), 2)
            .await;
        assert!(second.is_none());

        // but a withdrawal of 1 succeeds
        let third = manager
            .get_next_unspent_ticketbook_and_update(typ.clone(), 1)
            .await;
        assert!(third.is_some());

        // and now nothing left
        let fourth = manager.get_next_unspent_ticketbook_and_update(typ, 1).await;
        assert!(fourth.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn get_next_unspent_ticketbook_filters_by_type() -> anyhow::Result<()> {
        let manager = MemoryEcachTicketbookManager::new();
        let ticketbook = mock_ticketbook()?;

        manager.insert_new_ticketbook(&ticketbook, 5, 0).await;

        let mismatched = manager
            .get_next_unspent_ticketbook_and_update("nonexistent_type".to_string(), 1)
            .await;
        assert!(mismatched.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn revert_ticketbook_withdrawal_resets_spent_only_when_expected_matches(
    ) -> anyhow::Result<()> {
        let manager = MemoryEcachTicketbookManager::new();
        let ticketbook = mock_ticketbook()?;
        let typ = ticketbook.ticketbook_type().to_string();

        manager.insert_new_ticketbook(&ticketbook, 10, 0).await;
        manager
            .get_next_unspent_ticketbook_and_update(typ.clone(), 4)
            .await
            .expect("should withdraw");

        // stale expected_current_total_spent — should be rejected
        assert!(!manager.revert_ticketbook_withdrawal(0, 4, 99).await);
        // spent_tickets unchanged
        let used_after_failed = manager.get_ticketbooks_info().await[0].used_tickets;
        assert_eq!(used_after_failed, 4);

        // matching expected — should succeed and restore
        assert!(manager.revert_ticketbook_withdrawal(0, 4, 4).await);
        let used_after_revert = manager.get_ticketbooks_info().await[0].used_tickets;
        assert_eq!(used_after_revert, 0);

        // unknown ticketbook_id is rejected
        assert!(!manager.revert_ticketbook_withdrawal(999, 1, 0).await);

        Ok(())
    }

    #[tokio::test]
    async fn pending_ticketbook_round_trip() {
        let manager = MemoryEcachTicketbookManager::new();
        let issuance = mock_issuance(7);
        let deposit_id = issuance.deposit_id() as i64;

        assert!(manager.get_pending_ticketbooks().await.is_empty());

        manager.insert_pending_ticketbook(&issuance).await;

        let pending = manager.get_pending_ticketbooks().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].pending_id, deposit_id);
        assert_eq!(
            pending[0].pending_ticketbook.deposit_id(),
            issuance.deposit_id()
        );

        manager.remove_pending_ticketbook(deposit_id).await;
        assert!(manager.get_pending_ticketbooks().await.is_empty());

        // removing a non-existent id is a no-op
        manager.remove_pending_ticketbook(999).await;
    }

    #[tokio::test]
    async fn emergency_credential_lifecycle() {
        let manager = MemoryEcachTicketbookManager::new();

        let cred_a = EmergencyCredentialContent {
            typ: "type-a".to_string(),
            content: vec![1, 2, 3],
            expiration: None,
        };
        let cred_b = EmergencyCredentialContent {
            typ: "type-a".to_string(),
            content: vec![4, 5, 6],
            expiration: None,
        };
        let cred_c = EmergencyCredentialContent {
            typ: "type-b".to_string(),
            content: vec![7, 8, 9],
            expiration: None,
        };

        assert!(manager.get_emergency_credential("type-a").await.is_none());

        manager.insert_emergency_credential(&cred_a).await;
        manager.insert_emergency_credential(&cred_b).await;
        manager.insert_emergency_credential(&cred_c).await;

        // get returns the first inserted entry for the type
        let first = manager.get_emergency_credential("type-a").await.unwrap();
        assert_eq!(first.id, 0);
        assert_eq!(first.data.content, vec![1, 2, 3]);

        // remove by id drops only that entry; type-a now exposes cred_b
        manager.remove_emergency_credential(0).await;
        let after_remove = manager.get_emergency_credential("type-a").await.unwrap();
        assert_eq!(after_remove.id, 1);
        assert_eq!(after_remove.data.content, vec![4, 5, 6]);

        // remove by type clears the bucket entirely
        manager.remove_emergency_credentials_of_type("type-a").await;
        assert!(manager.get_emergency_credential("type-a").await.is_none());

        // unrelated type is untouched
        assert!(manager.get_emergency_credential("type-b").await.is_some());
    }

    #[tokio::test]
    async fn master_verification_key_round_trip() {
        let manager = MemoryEcachTicketbookManager::new();
        let key = mock_verification_key();
        let epoch = EpochVerificationKey {
            epoch_id: 7,
            key: key.clone(),
        };

        assert!(manager.get_master_verification_key(7).await.is_none());

        manager.insert_master_verification_key(&epoch).await;

        assert_eq!(manager.get_master_verification_key(7).await, Some(key));
        assert!(manager.get_master_verification_key(8).await.is_none());
    }

    #[tokio::test]
    async fn coin_index_signatures_round_trip() {
        let manager = MemoryEcachTicketbookManager::new();
        let sigs = AggregatedCoinIndicesSignatures {
            epoch_id: 3,
            signatures: vec![],
        };

        assert!(manager.get_coin_index_signatures(3).await.is_none());

        manager.insert_coin_index_signatures(&sigs).await;

        let retrieved = manager.get_coin_index_signatures(3).await;
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_empty());
        assert!(manager.get_coin_index_signatures(4).await.is_none());
    }

    #[tokio::test]
    async fn expiration_date_signatures_round_trip() {
        let manager = MemoryEcachTicketbookManager::new();
        let date = nym_ecash_time::ecash_today().date();
        let sigs = AggregatedExpirationDateSignatures {
            epoch_id: 5,
            expiration_date: date,
            signatures: vec![],
        };

        assert!(manager
            .get_expiration_date_signatures(date, 5)
            .await
            .is_none());

        manager.insert_expiration_date_signatures(&sigs).await;

        let retrieved = manager.get_expiration_date_signatures(date, 5).await;
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_empty());

        // wrong epoch / wrong date → miss
        assert!(manager
            .get_expiration_date_signatures(date, 6)
            .await
            .is_none());
    }
}
