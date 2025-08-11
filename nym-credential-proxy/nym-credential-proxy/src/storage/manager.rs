// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::models::{
    BlindedShares, BlindedSharesStatus, MinimalWalletShare, RawCoinIndexSignatures,
    RawExpirationDateSignatures, RawVerificationKey, StorableEcashDeposit,
};
use nym_validator_client::nyxd::contract_traits::ecash_query_client::DepositId;
use time::{Date, OffsetDateTime};
use tracing::error;

#[derive(Clone)]
pub(crate) struct SqliteStorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

impl SqliteStorageManager {
    pub(crate) async fn load_blinded_shares_status_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Option<BlindedShares>, sqlx::Error> {
        let res = sqlx::query_as(
            r#"
                    SELECT *
                    FROM blinded_shares
                    WHERE id = ?;
                "#,
        )
        .bind(id)
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(res)
    }

    pub(crate) async fn load_wallet_shares_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Vec<MinimalWalletShare>, sqlx::Error> {
        sqlx::query_as!(
            MinimalWalletShare,
            r#"
                SELECT t1.node_id, t1.blinded_signature, t1.epoch_id, t1.expiration_date as "expiration_date!: Date"
                FROM partial_blinded_wallet as t1
                JOIN ecash_deposit_usage as t2
                    on t1.corresponding_deposit = t2.deposit_id
                JOIN blinded_shares as t3
                    ON t2.request_uuid = t3.request_uuid
                WHERE t3.id = ?;
            "#,
            id
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn load_shares_error_by_device_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Option<String>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT error_message
                FROM blinded_shares
                WHERE id = ?;
            "#,
            id,
        )
        .fetch_one(&self.connection_pool)
        .await?
        .error_message)
    }

    pub(crate) async fn load_blinded_shares_status_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Option<BlindedShares>, sqlx::Error> {
        let res = sqlx::query_as(
            r#"
                    SELECT *
                    FROM blinded_shares
                    WHERE device_id = ? AND credential_id = ?;
                "#,
        )
        .bind(device_id)
        .bind(credential_id)
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(res)
    }

    pub(crate) async fn load_wallet_shares_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Vec<MinimalWalletShare>, sqlx::Error> {
        // https://docs.rs/sqlx/latest/sqlx/macro.query.html#force-a-differentcustom-type
        sqlx::query_as!(
            MinimalWalletShare,
            r#"
                SELECT
                    t1.node_id as "node_id!",
                    t1.blinded_signature as "blinded_signature!",
                    t1.epoch_id as "epoch_id!",
                    t1.expiration_date as "expiration_date!: Date"
                FROM partial_blinded_wallet as t1
                JOIN ecash_deposit_usage as t2
                    on t1.corresponding_deposit = t2.deposit_id
                JOIN blinded_shares as t3
                    ON t2.request_uuid = t3.request_uuid
                WHERE t3.device_id = ? AND t3.credential_id = ?;
            "#,
            device_id,
            credential_id
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn load_shares_error_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT error_message
                FROM blinded_shares
                WHERE device_id = ? AND credential_id = ?;
            "#,
            device_id,
            credential_id
        )
        .fetch_one(&self.connection_pool)
        .await?
        .error_message)
    }

    pub(crate) async fn insert_new_pending_async_shares_request(
        &self,
        request: String,
        device_id: &str,
        credential_id: &str,
    ) -> Result<BlindedShares, sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let res = sqlx::query_as(
            r#"
                INSERT INTO blinded_shares (status, request_uuid, device_id, credential_id, created, updated)
                VALUES (?, ?, ?, ?, ?, ?)
                RETURNING *
            "#,
        )
        .bind(BlindedSharesStatus::Pending)
        .bind(request)
        .bind(device_id)
        .bind(credential_id)
        .bind(now)
        .bind(now)
        .fetch_one(&self.connection_pool)
        .await?;

        Ok(res)
    }

    pub(crate) async fn update_pending_async_blinded_shares_issued(
        &self,
        available_shares: i64,
        device_id: &str,
        credential_id: &str,
    ) -> Result<BlindedShares, sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let res = sqlx::query_as(
            r#"
                    UPDATE blinded_shares
                    SET status = ?, updated = ?, error_message = NULL, available_shares = ?
                    WHERE device_id = ? AND credential_id = ?
                    RETURNING *;
                "#,
        )
        .bind(BlindedSharesStatus::Issued)
        .bind(now)
        .bind(available_shares)
        .bind(device_id)
        .bind(credential_id)
        .fetch_one(&self.connection_pool)
        .await?;

        Ok(res)
    }

    pub(crate) async fn update_pending_async_blinded_shares_error(
        &self,
        available_shares: i64,
        device_id: &str,
        credential_id: &str,
        error: &str,
    ) -> Result<BlindedShares, sqlx::Error> {
        let now = time::OffsetDateTime::now_utc();
        let res = sqlx::query_as(
            r#"
                    UPDATE blinded_shares
                    SET status = ?, error_message = ?, updated = ?, available_shares = ?
                    WHERE device_id = ? AND credential_id = ?
                    RETURNING *;
            "#,
        )
        .bind(BlindedSharesStatus::Error)
        .bind(error)
        .bind(now)
        .bind(available_shares)
        .bind(device_id)
        .bind(credential_id)
        .fetch_one(&self.connection_pool)
        .await?;

        Ok(res)
    }

    pub(crate) async fn prune_old_blinded_shares(
        &self,
        delete_after: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                DELETE FROM blinded_shares WHERE created < ?
            "#,
            delete_after,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn prune_old_partial_blinded_wallets(
        &self,
        delete_after: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                DELETE FROM partial_blinded_wallet WHERE created < ?
            "#,
            delete_after,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn prune_old_partial_blinded_wallet_failures(
        &self,
        delete_after: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                DELETE FROM partial_blinded_wallet_failure WHERE created < ?
            "#,
            delete_after,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_master_verification_key(
        &self,
        epoch_id: i64,
    ) -> Result<Option<RawVerificationKey>, sqlx::Error> {
        sqlx::query_as!(
            RawVerificationKey,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_key, serialization_revision as "serialization_revision: u8"
                FROM master_verification_key WHERE epoch_id = ?
            "#,
            epoch_id
        )
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_master_verification_key(
        &self,
        serialisation_revision: u8,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO master_verification_key(epoch_id, serialised_key, serialization_revision) VALUES (?, ?, ?)",
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_master_coin_index_signatures(
        &self,
        epoch_id: i64,
    ) -> Result<Option<RawCoinIndexSignatures>, sqlx::Error> {
        sqlx::query_as!(
            RawCoinIndexSignatures,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_signatures, serialization_revision as "serialization_revision: u8"
                FROM global_coin_index_signatures WHERE epoch_id = ?
            "#,
            epoch_id
        )
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_master_coin_index_signatures(
        &self,
        serialisation_revision: u8,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO global_coin_index_signatures(epoch_id, serialised_signatures, serialization_revision) VALUES (?, ?, ?)",
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error> {
        sqlx::query_as!(
            RawExpirationDateSignatures,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_signatures, serialization_revision as "serialization_revision: u8"
                FROM global_expiration_date_signatures
                WHERE expiration_date = ?
            "#,
            expiration_date
        )
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_master_expiration_date_signatures(
        &self,
        serialisation_revision: u8,
        epoch_id: i64,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO global_expiration_date_signatures(expiration_date, epoch_id, serialised_signatures, serialization_revision)
                VALUES (?, ?, ?, ?)
            "#,
            expiration_date,
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_new_deposits(
        &self,
        deposits: Vec<StorableEcashDeposit>,
    ) -> Result<(), sqlx::Error> {
        if deposits.is_empty() {
            // this should NEVER happen
            error!("attempted to insert empty list of deposits");
            return Ok(());
        }

        let mut query_builder =
            sqlx::QueryBuilder::new("INSERT INTO ecash_deposit (deposit_id, deposit_tx_hash, requested_on, deposit_amount, ed25519_deposit_private_key) ");

        query_builder.push_values(&deposits, |mut b, deposit| {
            b.push_bind(deposit.deposit_id)
                .push_bind(deposit.deposit_tx_hash.clone())
                .push_bind(deposit.requested_on)
                .push_bind(deposit.deposit_amount.clone())
                .push_bind(deposit.ed25519_deposit_private_key.as_ref());
        });

        query_builder.build().execute(&self.connection_pool).await?;
        Ok(())
    }

    pub(crate) async fn load_unused_deposits(
        &self,
    ) -> Result<Vec<StorableEcashDeposit>, sqlx::Error> {
        // select all entries from ecash_deposit where there is NO associated marked usage
        sqlx::query_as(
            r#"
                SELECT ecash_deposit.*
                FROM ecash_deposit ecash_deposit
                LEFT JOIN ecash_deposit_usage deposit_usage
                    ON ecash_deposit.deposit_id = deposit_usage.deposit_id
                WHERE deposit_usage.deposit_id IS NULL;
            "#,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn insert_deposit_usage(
        &self,
        deposit_id: DepositId,
        requested_on: OffsetDateTime,
        client_pubkey: Vec<u8>,
        request_uuid: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ecash_deposit_usage (deposit_id, ticketbooks_requested_on, client_pubkey, request_uuid)
                VALUES (?, ?, ?, ?)
            "#,
            deposit_id,
            requested_on,
            client_pubkey,
            request_uuid
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_deposit_usage_error(
        &self,
        deposit_id: DepositId,
        error: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE ecash_deposit_usage
                SET ticketbook_request_error = ?
                WHERE deposit_id = ?
            "#,
            error,
            deposit_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_partial_wallet_share(
        &self,
        deposit_id: DepositId,
        epoch_id: i64,
        expiration_date: Date,
        node_id: i64,
        created: OffsetDateTime,
        share: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO partial_blinded_wallet(corresponding_deposit, epoch_id, expiration_date, node_id, created, blinded_signature)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
                deposit_id,
                epoch_id,
                expiration_date,
                node_id,
                created,
                share
        )
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }

    pub(crate) async fn insert_partial_wallet_issuance_failure(
        &self,
        deposit_id: DepositId,
        epoch_id: i64,
        expiration_date: Date,
        node_id: i64,
        created: OffsetDateTime,
        failure_message: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO partial_blinded_wallet_failure(corresponding_deposit, epoch_id, expiration_date, node_id, created, failure_message)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
                deposit_id,
                epoch_id,
                expiration_date,
                node_id,
                created,
                failure_message
        )
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}
