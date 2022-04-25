// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::ERC20Credential;

#[derive(Clone)]
pub(crate) struct ERC20CredentialManager {
    connection_pool: sqlx::SqlitePool,
}

impl ERC20CredentialManager {
    /// Creates new instance of the `ERC20CredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        ERC20CredentialManager { connection_pool }
    }

    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `public_key`: Base58 representation of a public key.
    /// * `private_key`: Base58 representation of a private key.
    pub(crate) async fn insert_erc20_credential(
        &self,
        public_key: String,
        private_key: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO erc20_credentials(public_key, private_key, consumed) VALUES (?, ?, ?)",
            public_key,
            private_key,
            false,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub(crate) async fn get_next_erc20_credential(&self) -> Result<ERC20Credential, sqlx::Error> {
        sqlx::query_as!(
            ERC20Credential,
            "SELECT * FROM erc20_credentials WHERE consumed = false"
        )
        .fetch_one(&self.connection_pool)
        .await
    }

    /// Mark a credential as being consumed.
    pub(crate) async fn consume_erc20_credential(
        &self,
        public_key: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE erc20_credentials
                SET consumed = true
                WHERE public_key = ?
            "#,
            public_key
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }
}
