// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::path::PathBuf;
use tracing::info;

#[derive(Serialize, Deserialize)]
pub struct CachedState {
    pub importer_address: AccountId,
    pub state_imported: bool,
}

impl CachedState {
    pub fn save(&self) -> anyhow::Result<()> {
        let path = cached_state_file();
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let file = File::create(&path)?;
        serde_json::to_writer_pretty(file, self)?;

        info!("saved cached details to {}", path.display());
        Ok(())
    }

    pub fn load() -> anyhow::Result<Self> {
        let file = File::open(cached_state_file())?;
        Ok(serde_json::from_reader(&file)?)
    }
}

pub fn cached_state_file() -> PathBuf {
    dirs::cache_dir()
        .unwrap()
        .join("contract-state-importer")
        .join(".state.json")
}
