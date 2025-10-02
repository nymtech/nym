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

    async fn close(&self) {
        // nothing to do here
    }

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
        self.storage_manager
            .insert_new_ticketbook(
                ticketbook,
                ticketbook.params_total_tickets() as u32,
                ticketbook.spent_tickets() as u32,
            )
            .await;
        Ok(())
    }

    async fn insert_partial_issued_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
        allowed_start_ticket_index: u32,
        allowed_final_ticket_index: u32,
    ) -> Result<(), Self::StorageError> {
        // sanity check: start <= final && final <= params max
        if allowed_start_ticket_index > allowed_final_ticket_index {
            return Err(StorageError::database_inconsistency(
                "start_ticket_index must be less than or equal to final_ticket_index",
            ));
        }

        if allowed_final_ticket_index > ticketbook.params_total_tickets() as u32 {
            return Err(StorageError::database_inconsistency(
                "final ticket index must be less than or equal to params_total_tickets()",
            ));
        }

        self.storage_manager
            .insert_new_ticketbook(
                ticketbook,
                allowed_final_ticket_index + 1,
                allowed_start_ticket_index,
            )
            .await;
        Ok(())
    }

    async fn contains_issued_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
    ) -> Result<bool, StorageError> {
        Ok(self.storage_manager.contains_ticketbook(ticketbook).await)
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

    /// Tries to retrieve one of the stored ticketbook for the specified type,
    /// that has not yet expired and has required number of unspent tickets.
    /// it immediately updated the on-disk number of used tickets so that another task
    /// could obtain their own tickets at the same time
    async fn get_next_unspent_usable_ticketbook(
        &self,
        ticketbook_type: String,
        tickets: u32,
    ) -> Result<Option<RetrievedTicketbook>, Self::StorageError> {
        Ok(self
            .storage_manager
            .get_next_unspent_ticketbook_and_update(ticketbook_type, tickets)
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
        epoch_id: u64,
    ) -> Result<Option<Vec<AnnotatedExpirationDateSignature>>, Self::StorageError> {
        Ok(self
            .storage_manager
            .get_expiration_date_signatures(expiration_date, epoch_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use nym_compact_ecash::tests::helpers::generate_expiration_date_signatures;
    use nym_compact_ecash::{issue, ttp_keygen};
    use nym_credentials_interface::TicketType;
    use nym_crypto::asymmetric::ed25519;
    use nym_ecash_time::EcashTime;
    use nym_test_utils::helpers::deterministic_rng;

    fn mock_ticketbook() -> anyhow::Result<IssuedTicketBook> {
        let signing_keys = ttp_keygen(1, 1)?.remove(0);

        let deposit_id = 42;
        let identifier = "foomp";
        let mut rng = deterministic_rng();
        let key = ed25519::PrivateKey::new(&mut rng);
        let typ = TicketType::V1MixnetEntry;

        let issuance = IssuanceTicketBook::new(deposit_id, identifier, key, typ);
        let expiration_date = issuance.expiration_date();

        let sig_req = issuance.prepare_for_signing();
        let _exp_date_sigs = generate_expiration_date_signatures(
            sig_req.expiration_date.ecash_unix_timestamp(),
            &[signing_keys.secret_key()],
            &vec![signing_keys.verification_key()],
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
            &vec![partial_wallet],
            sig_req,
        )?;

        Ok(issuance.into_issued_ticketbook(wallet, 1))
    }

    #[tokio::test]
    async fn storing_partial_ticketbook() -> anyhow::Result<()> {
        let storage = EphemeralStorage::default();
        let ticketbook = mock_ticketbook()?;
        let typ = ticketbook.ticketbook_type();

        storage
            .insert_partial_issued_ticketbook(&ticketbook, 5, 5)
            .await?;
        let retrieved = storage
            .get_next_unspent_usable_ticketbook(typ.to_string(), 1)
            .await?;
        assert!(retrieved.is_some());
        let val = retrieved.unwrap();
        assert_eq!(val.total_tickets, 6);
        assert_eq!(val.ticketbook.spent_tickets(), 5);

        // we only had 1 ticket
        let retrieved2 = storage
            .get_next_unspent_usable_ticketbook(typ.to_string(), 1)
            .await?;
        assert!(retrieved2.is_none());

        let _another = mock_ticketbook()?;
        let typ = ticketbook.ticketbook_type();

        // 3 tickets (4, 5, 6)
        storage
            .insert_partial_issued_ticketbook(&ticketbook, 4, 6)
            .await?;
        assert!(storage
            .get_next_unspent_usable_ticketbook(typ.to_string(), 1)
            .await?
            .is_some());
        assert!(storage
            .get_next_unspent_usable_ticketbook(typ.to_string(), 1)
            .await?
            .is_some());
        assert!(storage
            .get_next_unspent_usable_ticketbook(typ.to_string(), 1)
            .await?
            .is_some());
        assert!(storage
            .get_next_unspent_usable_ticketbook(typ.to_string(), 1)
            .await?
            .is_none());

        Ok(())
    }
}
