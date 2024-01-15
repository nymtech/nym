// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::default_data_directory;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_SCRAPER_DB_FILENAME: &str = "nyxd_blocks.sqlite";
pub const DEFAULT_REWARD_HISTORY_DB_FILENAME: &str = "rewards.sqlite";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidatorRewarderPaths {
    pub nyxd_scraper: PathBuf,

    pub reward_history: PathBuf,
}

impl Default for ValidatorRewarderPaths {
    fn default() -> Self {
        ValidatorRewarderPaths {
            nyxd_scraper: default_data_directory().join(DEFAULT_SCRAPER_DB_FILENAME),
            reward_history: default_data_directory().join(DEFAULT_REWARD_HISTORY_DB_FILENAME),
        }
    }
}
