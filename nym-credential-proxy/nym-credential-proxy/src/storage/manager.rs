// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::storage::models::{
    BlindedShares, BlindedSharesStatus, MinimalWalletShare, RawCoinIndexSignatures,
    RawExpirationDateSignatures, RawVerificationKey,
};
use nym_validator_client::nyxd::contract_traits::ecash_query_client::DepositId;
use time::{Date, OffsetDateTime};

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
                JOIN ticketbook_deposit as t2
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
                SELECT t1.node_id, t1.blinded_signature, t1.epoch_id, t1.expiration_date as "expiration_date!: Date"
                FROM partial_blinded_wallet as t1
                JOIN ticketbook_deposit as t2
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
    ) -> Result<BlindedShares, VpnApiError> {
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
    ) -> Result<BlindedShares, VpnApiError> {
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
    ) -> Result<(), VpnApiError> {
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
    ) -> Result<(), VpnApiError> {
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
    ) -> Result<(), VpnApiError> {
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_deposit_data(
        &self,
        deposit_id: DepositId,
        deposit_tx_hash: String,
        requested_on: OffsetDateTime,
        request_uuid: String,
        deposit_amount: String,
        client_pubkey: &[u8],
        deposit_ed25519_private_key: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ticketbook_deposit(deposit_id, deposit_tx_hash, requested_on, request_uuid, deposit_amount, client_pubkey, ed25519_deposit_private_key)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
                deposit_id,
                deposit_tx_hash,
                requested_on,
                request_uuid,
                deposit_amount,
                client_pubkey,
                deposit_ed25519_private_key,
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
