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
    /// * `signature`: Coconut credential in the form of a signature.
    pub(crate) async fn insert_coconut_credential(
        &self,
        credential: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO coconut_credentials(credential) VALUES (?)",
            credential,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub(crate) async fn get_next_coconut_credential(
        &self,
    ) -> Result<Option<CoconutCredential>, sqlx::Error> {
        sqlx::query_as!(
            CoconutCredential,
            "SELECT * FROM coconut_credentials LIMIT 1"
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Removes from the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `signature`: Coconut credential in the form of a signature.
    pub(crate) async fn remove_coconut_credential(
        &self,
        credential: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM coconut_credentials WHERE credential = ?",
            credential
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
