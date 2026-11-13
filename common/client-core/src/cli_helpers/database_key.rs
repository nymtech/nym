// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sqlcipher::DatabaseKey;

/// SQLCipher key encrypting the client's databases, flattened into every command that opens them.
#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct DatabaseKeyArgs {
    /// Raw 256-bit key, as 64 hex characters, encrypting this client's databases with SQLCipher.
    /// Databases still stored in plaintext are encrypted the first time they are opened with it,
    /// and the same key must then be provided on every run. Requires a build with SQLCipher.
    /// Prefer the environment variable: a flag value is visible in the process list. It must be
    /// set in the process environment, as `--config-env-file` is only read after the arguments
    /// are parsed.
    #[cfg_attr(
        feature = "cli",
        clap(long, env = "NYM_CLIENT_DATABASE_KEY", hide_env_values = true)
    )]
    pub database_key: Option<DatabaseKey>,
}
