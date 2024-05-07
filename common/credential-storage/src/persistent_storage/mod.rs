// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backends::sqlite::{
    get_next_unspent_ticketbook, increase_used_ticketbook_tickets, SqliteEcashTicketbookManager,
};
use crate::error::StorageError;
use crate::models::{BasicTicketbookInformation, RetrievedPendingTicketbook, RetrievedTicketbook};
use crate::persistent_storage::helpers::{
    deserialise_coin_index_signatures, deserialise_expiration_date_signatures,
    serialise_coin_index_signatures, serialise_expiration_date_signatures,
};
use crate::storage::Storage;
use async_trait::async_trait;
use log::{debug, error};
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
use nym_ecash_time::{ecash_today, Date, EcashTime};
use sqlx::ConnectOptions;
use std::path::Path;
use zeroize::Zeroizing;

mod helpers;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct PersistentStorage {
    storage_manager: SqliteEcashTicketbookManager,
}

impl PersistentStorage {
    /// Initialises `PersistentStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        debug!(
            "Attempting to connect to database {:?}",
            database_path.as_ref().as_os_str()
        );

        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to perform migration on the SQLx database: {err}");
            return Err(err.into());
        }

        Ok(PersistentStorage {
            storage_manager: SqliteEcashTicketbookManager::new(connection_pool.clone()),
        })
    }
}

#[async_trait]
impl Storage for PersistentStorage {
    type StorageError = StorageError;

    /// remove all expired ticketbooks and expiration date signatures
    async fn cleanup_expired(&self) -> Result<(), Self::StorageError> {
        let ecash_yesterday = ecash_today().date().previous_day().unwrap();
        self.storage_manager
            .cleanup_expired(ecash_yesterday)
            .await?;
        Ok(())
    }

    async fn insert_pending_ticketbook(
        &self,
        ticketbook: &IssuanceTicketBook,
    ) -> Result<(), Self::StorageError> {
        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        let serialisation_revision = ser.revision;

        self.storage_manager
            .insert_pending_ticketbook(
                serialisation_revision,
                ticketbook.deposit_id(),
                &data,
                ticketbook.expiration_date(),
            )
            .await?;

        Ok(())
    }

    async fn insert_issued_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
    ) -> Result<(), Self::StorageError> {
        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        let serialisation_revision = ser.revision;

        self.storage_manager
            .insert_new_ticketbook(
                serialisation_revision,
                &data,
                ticketbook.expiration_date(),
                ticketbook.epoch_id() as u32,
                ticketbook.params_total_tickets() as u32,
                ticketbook.spent_tickets() as u32,
            )
            .await?;

        Ok(())
    }

    async fn get_ticketbooks_info(
        &self,
    ) -> Result<Vec<BasicTicketbookInformation>, Self::StorageError> {
        Ok(self.storage_manager.get_ticketbooks_info().await?)
    }

    async fn get_pending_ticketbooks(
        &self,
    ) -> Result<Vec<RetrievedPendingTicketbook>, Self::StorageError> {
        let pending = self
            .storage_manager
            .get_pending_ticketbooks()
            .await?
            .into_iter()
            .map(|p| {
                IssuanceTicketBook::try_unpack(&p.pending_ticketbook_data, p.serialization_revision)
                    .map_err(|err| {
                        StorageError::database_inconsistency(format!(
                            "failed to deserialise stored pending ticketbook: {err}"
                        ))
                    })
                    .map(|pending_ticketbook| RetrievedPendingTicketbook {
                        pending_id: p.deposit_id,
                        pending_ticketbook,
                    })
            })
            .collect::<Result<_, _>>()?;
        Ok(pending)
    }

    async fn remove_pending_ticketbook(&self, pending_id: i64) -> Result<(), Self::StorageError> {
        self.storage_manager
            .remove_pending_ticketbook(pending_id)
            .await?;
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
        let deadline = ecash_today().ecash_date();
        let mut tx = self.storage_manager.begin_storage_tx().await?;

        // we don't want ticketbooks with expiration in the past
        let Some(raw) = get_next_unspent_ticketbook(&mut tx, deadline, tickets).await? else {
            // make sure to finish our tx
            tx.commit().await?;
            return Ok(None);
        };

        let mut deserialised =
            IssuedTicketBook::try_unpack(&raw.ticketbook_data, raw.serialization_revision)
                .map_err(|err| {
                    StorageError::database_inconsistency(format!(
                        "failed to deserialise stored ticketbook: {err}"
                    ))
                })?;

        increase_used_ticketbook_tickets(&mut tx, raw.id, tickets).await?;
        tx.commit().await?;

        // set the number of spent tickets on the crypto object
        // TODO: I don't like how that's required and can be easily missed,
        // perhaps we shouldn't be storing the `IssuedTicketBook` data in the db,
        // but all of its fields instead?
        deserialised.update_spent_tickets(raw.used_tickets as u64);
        Ok(Some(RetrievedTicketbook {
            ticketbook_id: raw.id,
            ticketbook: deserialised,
        }))
    }

    async fn attempt_revert_ticketbook_withdrawal(
        &self,
        ticketbook_id: i64,
        withdrawn: u32,
        expected_current_total_spent: u32,
    ) -> Result<bool, Self::StorageError> {
        Ok(self
            .storage_manager
            .decrease_used_ticketbook_tickets(
                ticketbook_id,
                withdrawn,
                expected_current_total_spent,
            )
            .await?)
    }

    async fn get_master_verification_key(
        &self,
        epoch_id: u64,
    ) -> Result<Option<VerificationKeyAuth>, Self::StorageError> {
        let Some(raw) = self
            .storage_manager
            .get_master_verification_key(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let master_vk = VerificationKeyAuth::from_bytes(&raw).map_err(|_| {
            StorageError::database_inconsistency("malformed stored master verification key")
        })?;

        Ok(Some(master_vk))
    }

    async fn insert_master_verification_key(
        &self,
        epoch_id: u64,
        key: &VerificationKeyAuth,
    ) -> Result<(), Self::StorageError> {
        Ok(self
            .storage_manager
            .insert_master_verification_key(epoch_id as i64, &key.to_bytes())
            .await?)
    }

    async fn get_coin_index_signatures(
        &self,
        epoch_id: u64,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, Self::StorageError> {
        let Some(raw) = self
            .storage_manager
            .get_coin_index_signatures(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(deserialise_coin_index_signatures(&raw)?))
    }

    async fn insert_coin_index_signatures(
        &self,
        epoch_id: u64,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), Self::StorageError> {
        self.storage_manager
            .insert_coin_index_signatures(epoch_id as i64, &serialise_coin_index_signatures(sigs))
            .await?;
        Ok(())
    }

    async fn get_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<Vec<AnnotatedExpirationDateSignature>>, Self::StorageError> {
        let Some(raw) = self
            .storage_manager
            .get_expiration_date_signatures(expiration_date)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(deserialise_expiration_date_signatures(
            &raw.serialised_signatures,
        )?))
    }

    async fn insert_expiration_date_signatures(
        &self,
        epoch_id: u64,
        expiration_date: Date,
        sigs: &[AnnotatedExpirationDateSignature],
    ) -> Result<(), Self::StorageError> {
        self.storage_manager
            .insert_expiration_date_signatures(
                epoch_id as i64,
                expiration_date,
                &serialise_expiration_date_signatures(sigs),
            )
            .await?;
        Ok(())
    }
}
