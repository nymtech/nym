// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::CachedState;
use anyhow::bail;
use nym_validator_client::nyxd::AccountId;
use std::env::current_dir;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

// this only works if the cli is called from somewhere within the nym directory
// (which realistically is going to be the case most of the time)
pub fn importer_contract_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(explicit) = explicit {
        return Ok(explicit);
    }

    for ancestor in current_dir()?.ancestors() {
        debug!("checking {:?}", fs::canonicalize(ancestor));
        for content in ancestor.read_dir()? {
            let dir_entry = content?;
            let Ok(name) = dir_entry.file_name().into_string() else {
                continue;
            };

            if name == "target" {
                let maybe_contract_path = dir_entry
                    .path()
                    .join("wasm32-unknown-unknown")
                    .join("release")
                    .join("importer_contract.wasm");

                if maybe_contract_path.exists() {
                    return Ok(maybe_contract_path);
                }
            }
        }
    }

    bail!("could not find importer_contract.wasm")
}

pub fn importer_contract_address(explicit: Option<AccountId>) -> anyhow::Result<AccountId> {
    if let Some(explicit) = explicit {
        return Ok(explicit);
    }

    let state = CachedState::load()?;
    Ok(state.importer_address)
}
