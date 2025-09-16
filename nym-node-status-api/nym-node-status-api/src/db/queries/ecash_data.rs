// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::Storage;
use nym_credential_proxy_lib::storage::models::{
    RawCoinIndexSignatures, RawExpirationDateSignatures, RawVerificationKey,
};
use time::Date;
use tracing::error;

impl Storage {
    pub(crate) async fn available_tickets_of_type(&self, typ: &str) -> Result<i64, sqlx::Error> {
        let count = sqlx::query!(
            r#"
                SELECT SUM(total_tickets - used_tickets) AS available_tickets
                FROM ecash_ticketbook
                WHERE ticketbook_type = $1;
            "#,
            typ
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count.available_tickets.unwrap_or_default())
    }

    pub(crate) async fn insert_pending_ticketbook(
        &self,
        serialisation_revision: i16,
        deposit_id: i32,
        data: &[u8],
        expiration_date: Date,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO pending_issuance
                (deposit_id, serialization_revision, pending_ticketbook_data, expiration_date)
                VALUES ($1, $2, $3, $4)
            "#,
            deposit_id,
            serialisation_revision,
            data,
            expiration_date,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_new_ticketbook(
        &self,
        serialisation_revision: i16,
        data: &[u8],
        expiration_date: Date,
        typ: &str,
        epoch_id: i32,
        total_tickets: i32,
        used_tickets: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ecash_ticketbook
                (serialization_revision, ticketbook_data, expiration_date, ticketbook_type, epoch_id, total_tickets, used_tickets)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            serialisation_revision,
            data,
            expiration_date,
            typ,
            epoch_id,
            total_tickets,
            used_tickets,
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub(crate) async fn get_master_verification_key(
        &self,
        epoch_id: i32,
    ) -> Result<Option<RawVerificationKey>, sqlx::Error> {
        sqlx::query!(
            r#"
                SELECT epoch_id as "epoch_id: i32", serialised_key, serialization_revision as "serialization_revision: i16"
                FROM master_verification_key WHERE epoch_id = $1
            "#,
            epoch_id
        )
        .fetch_optional(&self.pool)
        .await.map(|maybe_row| {
            maybe_row.map(|row| {
                RawVerificationKey {
                    epoch_id: row.epoch_id.try_into()
                        .inspect_err(|err| error!("failed to convert i32 epoch_id into u32: {err}"))
                        .unwrap_or_default(),
                    serialised_key: row.serialised_key,
                    serialization_revision: row.serialization_revision.try_into()
                        .inspect_err(|err| error!("failed to convert i16 serialization_revision into u8: {err}"))
                        .unwrap_or_default(),
                }
            })
        })
    }

    pub(crate) async fn insert_master_verification_key(
        &self,
        serialisation_revision: i16,
        epoch_id: i32,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO master_verification_key(epoch_id, serialised_key, serialization_revision) VALUES ($1, $2, $3)",
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_master_coin_index_signatures(
        &self,
        epoch_id: i32,
    ) -> Result<Option<RawCoinIndexSignatures>, sqlx::Error> {
        sqlx::query!(
            r#"
                SELECT epoch_id as "epoch_id: i32", serialised_signatures, serialization_revision as "serialization_revision: i16"
                FROM global_coin_index_signatures WHERE epoch_id = $1
            "#,
            epoch_id
         )
        .fetch_optional(&self.pool)
        .await.map(|maybe_row| {
            maybe_row.map(|row| {
                RawCoinIndexSignatures {
                    epoch_id: row.epoch_id.try_into()
                        .inspect_err(|err| error!("failed to convert i32 epoch_id into u32: {err}"))
                        .unwrap_or_default(),
                    serialised_signatures: row.serialised_signatures,
                    serialization_revision: row.serialization_revision.try_into()
                        .inspect_err(|err| error!("failed to convert i16 serialization_revision into u8: {err}"))
                        .unwrap_or_default(),
                }
            })
        })
    }

    pub(crate) async fn insert_master_coin_index_signatures(
        &self,
        serialisation_revision: i16,
        epoch_id: i32,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO global_coin_index_signatures(epoch_id, serialised_signatures, serialization_revision) VALUES ($1, $2, $3)",
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: i32,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error> {
        sqlx::query!(
            r#"
                SELECT serialised_signatures, serialization_revision as "serialization_revision: i16"
                FROM global_expiration_date_signatures
                WHERE expiration_date = $1 AND epoch_id = $2
            "#,
            expiration_date,
            epoch_id
        )
            .fetch_optional(&self.pool)
            .await.map(|maybe_row| {
            maybe_row.map(|row| {
                RawExpirationDateSignatures {
                    serialised_signatures: row.serialised_signatures,
                    serialization_revision: row.serialization_revision.try_into()
                        .inspect_err(|err| error!("failed to convert i16 serialization_revision into u8: {err}"))
                        .unwrap_or_default(),
                }
            })
        })
    }

    pub(crate) async fn insert_master_expiration_date_signatures(
        &self,
        serialisation_revision: i16,
        epoch_id: i32,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO global_expiration_date_signatures(expiration_date, epoch_id, serialised_signatures, serialization_revision)
                VALUES ($1, $2, $3, $4)
            "#,
            expiration_date,
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
