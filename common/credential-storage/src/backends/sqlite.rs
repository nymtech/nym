// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{CoconutCredential, EcashCredential};

#[derive(Clone)]
pub struct CoconutCredentialManager {
    connection_pool: sqlx::SqlitePool,
}

impl CoconutCredentialManager {
    /// Creates new instance of the `CoconutCredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub fn new(connection_pool: sqlx::SqlitePool) -> Self {
        CoconutCredentialManager { connection_pool }
    }

    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `voucher_value`: Plaintext bandwidth value of the credential.
    /// * `voucher_info`: Plaintext information of the credential.
    /// * `serial_number`: Base58 representation of the serial number attribute.
    /// * `binding_number`: Base58 representation of the binding number attribute.
    /// * `signature`: Coconut credential in the form of a signature.
    pub async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
        epoch_id: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO coconut_credentials(voucher_value, voucher_info, serial_number, binding_number, signature, epoch_id, consumed) VALUES (?, ?, ?, ?, ?, ?, ?)",
            voucher_value, voucher_info, serial_number, binding_number, signature, epoch_id, false
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `voucher_info`: What type of credential it is.
    /// * `signature`: Ecash wallet credential in the form of a wallet.
    /// * `epoch_id`: The epoch when it was signed.

    pub async fn insert_ecash_credential(
        &self,
        voucher_info: String,
        wallet: String,
        epoch_id: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO ecash_credentials(voucher_info, wallet, epoch_id, consumed) VALUES (?, ?, ?, ?)",
            voucher_info, wallet, epoch_id, false
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_ecash_credential(&self) -> Result<Option<EcashCredential>, sqlx::Error> {
        sqlx::query_as!(
            EcashCredential,
            "SELECT * FROM ecash_credentials WHERE NOT consumed"
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_coconut_credential(
        &self,
    ) -> Result<Option<CoconutCredential>, sqlx::Error> {
        sqlx::query_as!(
            CoconutCredential,
            "SELECT * FROM coconut_credentials WHERE NOT consumed"
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Consumes in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Database id.
    pub async fn consume_coconut_credential(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE coconut_credentials SET consumed = TRUE WHERE id = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Consumes in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `wallet` : New wallet string to update with
    /// * `id`: Database id.
    /// * `consumed` : If the wallet is entirely consumed
    ///
    pub async fn update_ecash_credential(
        &self,
        wallet: String,
        id: i64,
        consumed: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE ecash_credentials SET wallet = ?, consumed = ? WHERE id = ?",
            wallet,
            consumed,
            id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
