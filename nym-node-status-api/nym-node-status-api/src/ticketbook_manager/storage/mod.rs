// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::Storage;
use crate::ticketbook_manager::storage::auxiliary_models::RetrievedTicketbook;
use anyhow::{Context, anyhow};
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::shared_state::ecash_state::{
    IssuanceTicketBook, IssuedTicketBook, TicketType,
};
use nym_credential_proxy_lib::storage::traits::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    GlobalEcashDataCache, VersionedSerialise,
};
use nym_crypto::aes::cipher::zeroize::Zeroizing;
use nym_ecash_time::{EcashTime, ecash_today};
use nym_validator_client::nym_api::EpochId;
use time::Date;

pub(crate) mod auxiliary_models;

#[derive(Clone)]
pub(crate) struct TicketbookManagerStorage {
    storage: Storage,
}

impl From<Storage> for TicketbookManagerStorage {
    fn from(storage: Storage) -> Self {
        TicketbookManagerStorage { storage }
    }
}

impl TicketbookManagerStorage {
    pub(crate) async fn available_tickets_of_type(&self, typ: TicketType) -> anyhow::Result<usize> {
        self.storage
            .available_tickets_of_type(&typ.to_string())
            .await?
            .try_into()
            .context("failed to convert ticket count from i64 to usize")
    }

    pub(crate) async fn insert_pending_ticketbook(
        &self,
        ticketbook: &IssuanceTicketBook,
    ) -> anyhow::Result<()> {
        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        let serialisation_revision = ser.revision;

        self.storage
            .insert_pending_ticketbook(
                serialisation_revision as i16,
                ticketbook.deposit_id() as i32,
                &data,
                ticketbook.expiration_date(),
            )
            .await?;

        Ok(())
    }

    pub(crate) async fn insert_issued_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
    ) -> anyhow::Result<()> {
        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        let serialisation_revision = ser.revision;

        self.storage
            .insert_new_ticketbook(
                serialisation_revision as i16,
                &data,
                ticketbook.expiration_date(),
                &ticketbook.ticketbook_type().to_string(),
                ticketbook.epoch_id() as i32,
                ticketbook.params_total_tickets() as i32,
                ticketbook.spent_tickets() as i32,
            )
            .await?;

        Ok(())
    }

    /// Tries to retrieve one of the stored ticketbook that has not yet expired
    /// it immediately updated the on-disk number of used tickets so that another task
    /// could obtain their own tickets at the same time
    pub(crate) async fn next_ticket(
        &self,
        ticket_type: TicketType,
        testrun_id: i32,
    ) -> anyhow::Result<Option<RetrievedTicketbook>> {
        let deadline = ecash_today().ecash_date();

        // we don't want ticketbooks with expiration in the past
        // note: this query updates the spent tickets atomically
        let Some(raw) = self
            .storage
            .get_next_unspent_ticketbook(ticket_type.to_string(), deadline)
            .await?
        else {
            return Ok(None);
        };

        let mut deserialised = IssuedTicketBook::try_unpack(
            &raw.ticketbook_data,
            u8::try_from(raw.serialization_revision)
                .context("failed to convert i16 serialization_revision into u8")?,
        )
        .map_err(|err| anyhow!("failed to deserialise stored ticketbook: {err}"))?;

        self.storage
            .set_distributed_ticketbook(testrun_id, raw.id, raw.used_tickets)
            .await?;

        deserialised.update_spent_tickets(raw.used_tickets as u64);
        Ok(Some(RetrievedTicketbook {
            ticketbook_id: raw.id,
            total_tickets: raw
                .total_tickets
                .try_into()
                .context("failed to convert i32 total tickets into u32")?,
            spent_tickets: deserialised.spent_tickets() as u32,
            ticketbook: deserialised,
        }))
    }
}

impl GlobalEcashDataCache for TicketbookManagerStorage {
    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochVerificationKey>, CredentialProxyError> {
        let Some(raw) = self
            .storage
            .get_master_verification_key(epoch_id as i32)
            .await?
        else {
            return Ok(None);
        };

        let deserialised =
            EpochVerificationKey::try_unpack(&raw.serialised_key, raw.serialization_revision)
                .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), CredentialProxyError> {
        let packed = key.pack();
        Ok(self
            .storage
            .insert_master_verification_key(
                packed.revision as i16,
                key.epoch_id as i32,
                &packed.data,
            )
            .await?)
    }

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        let Some(raw) = self
            .storage
            .get_master_coin_index_signatures(epoch_id as i32)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedCoinIndicesSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    async fn insert_master_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), CredentialProxyError> {
        let packed = signatures.pack();
        self.storage
            .insert_master_coin_index_signatures(
                packed.revision as i16,
                signatures.epoch_id as i32,
                &packed.data,
            )
            .await?;
        Ok(())
    }

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedExpirationDateSignatures>, CredentialProxyError> {
        let Some(raw) = self
            .storage
            .get_master_expiration_date_signatures(expiration_date, epoch_id as i32)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedExpirationDateSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    async fn insert_master_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), CredentialProxyError> {
        let packed = signatures.pack();
        self.storage
            .insert_master_expiration_date_signatures(
                packed.revision as i16,
                signatures.epoch_id as i32,
                signatures.expiration_date,
                &packed.data,
            )
            .await?;
        Ok(())
    }
}
