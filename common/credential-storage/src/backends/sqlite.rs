// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CoconutCredential;

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
}
