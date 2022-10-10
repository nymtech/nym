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
}
