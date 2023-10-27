// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Clone)]
pub(crate) struct CredentialManager {
    connection_pool: sqlx::SqlitePool,
}

impl CredentialManager {
    /// Creates new instance of the `CredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        CredentialManager { connection_pool }
    }

    /// Inserts provided credential into the database.
    /// If the credential previously existed for the provided client, they are overwritten with the new data.
    ///
    /// # Arguments
    ///
    /// * `credential`: base58 representation of an ecash credential
    pub(crate) async fn insert_credential(&self, credential: String) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT OR REPLACE INTO credentials(credentials) VALUES (?)",
            credential
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
