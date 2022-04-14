// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CoconutCredential;

#[derive(Clone)]
pub(crate) struct CoconutCredentialManager {
    connection_pool: sqlx::SqlitePool,
}

impl CoconutCredentialManager {
    /// Creates new instance of the `CoconutCredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
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
    pub(crate) async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO coconut_credentials(voucher_value, voucher_info, serial_number, binding_number, signature) VALUES (?, ?, ?, ?, ?)",
            voucher_value, voucher_info, serial_number, binding_number, signature
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub(crate) async fn get_next_coconut_credential(
        &self,
    ) -> Result<CoconutCredential, sqlx::Error> {
        sqlx::query_as!(CoconutCredential, "SELECT * FROM coconut_credentials")
            .fetch_one(&self.connection_pool)
            .await
    }

    /// Removes from the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Database id.
    pub(crate) async fn remove_coconut_credential(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM coconut_credentials WHERE id = ?", id)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }
}
