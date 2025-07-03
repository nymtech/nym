// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::default_data_directory;
use crate::error::NymRewarderError;
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_SCRAPER_DB_FILENAME: &str = "nyxd_blocks.sqlite";
pub const DEFAULT_REWARD_HISTORY_DB_FILENAME: &str = "rewards.sqlite";

pub const DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_identity";
pub const DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_identity.pub";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidatorRewarderPaths {
    pub nyxd_scraper: String,

    pub reward_history: PathBuf,

    /// Path to file containing private identity key of the rewarder.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing public identity key of the rewarder.
    pub public_ed25519_identity_key_file: PathBuf,
}

impl ValidatorRewarderPaths {
    pub fn load_ed25519_identity(&self) -> Result<ed25519::KeyPair, NymRewarderError> {
        let keypaths = nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        );

        nym_pemstore::load_keypair(&keypaths).map_err(|source| {
            NymRewarderError::Ed25519KeyLoadFailure {
                public_key_path: self.public_ed25519_identity_key_file.clone(),
                private_key_path: self.private_ed25519_identity_key_file.clone(),
                source,
            }
        })
    }
}

impl Default for ValidatorRewarderPaths {
    fn default() -> Self {
        ValidatorRewarderPaths {
            // validator rewarder uses sqlite
            nyxd_scraper: (default_data_directory().join(DEFAULT_SCRAPER_DB_FILENAME))
                .to_string_lossy()
                .to_string(),
            reward_history: default_data_directory().join(DEFAULT_REWARD_HISTORY_DB_FILENAME),
            private_ed25519_identity_key_file: default_data_directory()
                .join(DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: default_data_directory()
                .join(DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME),
        }
    }
}
