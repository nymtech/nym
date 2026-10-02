// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::persistence::Passphrase;

/// Passphrase protecting the client's private keys on disk, flattened into every command that touches them.
#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct KeyPassphraseArgs {
    /// Passphrase protecting this client's private keys on disk. Keys still stored in plaintext
    /// are encrypted the first time they are loaded with it.
    #[cfg_attr(
        feature = "cli",
        clap(
            long,
            env = "NYM_CLIENT_KEY_PASSPHRASE",
            hide_env_values = true,
            value_parser = parse_passphrase
        )
    )]
    pub key_passphrase: Option<Passphrase>,
}

#[cfg(feature = "cli")]
fn parse_passphrase(raw: &str) -> Result<Passphrase, String> {
    if raw.is_empty() {
        return Err("the key passphrase must not be empty".to_string());
    }
    Ok(Passphrase::new(raw))
}

#[cfg(all(test, feature = "cli"))]
mod tests {
    use super::*;

    #[test]
    fn empty_passphrase_is_rejected() {
        assert!(parse_passphrase("").is_err());
        assert!(parse_passphrase("hunter2").is_ok());
    }
}
