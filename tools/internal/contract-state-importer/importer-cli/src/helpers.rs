// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::CachedState;
use anyhow::bail;
use nym_validator_client::nyxd::cosmwasm_client::types::Model;
use nym_validator_client::nyxd::AccountId;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env::current_dir;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

pub fn importer_contract_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    contract_path("importer_contract.wasm", explicit)
}

pub fn mixnet_contract_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    contract_path("mixnet_contract.wasm", explicit)
}

pub fn vesting_contract_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    contract_path("vesting_contract.wasm", explicit)
}

fn find_contract_in(root: PathBuf, target_name: &str) -> anyhow::Result<Option<PathBuf>> {
    for content in root.read_dir()? {
        let dir_entry = content?;

        let path = dir_entry.path();
        debug!("checking {:?}", fs::canonicalize(path.clone()));

        let Ok(name) = dir_entry.file_name().into_string() else {
            continue;
        };

        if name == "target" {
            let maybe_contract_path = dir_entry
                .path()
                .join("wasm32-unknown-unknown")
                .join("release")
                .join(target_name);

            if maybe_contract_path.exists() {
                return Ok(Some(maybe_contract_path));
            }
        }

        if path.is_dir() {
            if let Some(found) = find_contract_in(path, target_name)? {
                return Ok(Some(found));
            }
        }
    }

    Ok(None)
}

// this only works if the cli is called from somewhere within the nym directory
// (which realistically is going to be the case most of the time)
pub fn contract_path(target_name: &str, explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
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
                    .join(target_name);

                if maybe_contract_path.exists() {
                    return Ok(maybe_contract_path);
                }
            }

            // SPECIAL CASE:
            // if there's a `contracts` directory, do traverse its children
            if name == "contracts" {
                if let Some(contract) = find_contract_in(dir_entry.path(), target_name)? {
                    return Ok(contract);
                }
            }
        }
    }

    bail!("could not find {target_name}")
}

pub fn importer_contract_address(explicit: Option<AccountId>) -> anyhow::Result<AccountId> {
    if let Some(explicit) = explicit {
        return Ok(explicit);
    }

    let state = CachedState::load()?;
    Ok(state.importer_address)
}

pub fn find_value<T>(raw_state: &[Model], key: &[u8]) -> anyhow::Result<Option<T>>
where
    T: DeserializeOwned,
{
    let Some(entry) = raw_state.iter().find(|kv| kv.key == key) else {
        return Ok(None);
    };

    Ok(Some(serde_json::from_slice(&entry.value)?))
}

pub fn update_value<T>(raw_state: &mut [Model], key: &[u8], value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    let Some(entry) = raw_state.iter_mut().find(|kv| kv.key == key) else {
        bail!("couldn't find corresponding state entry")
    };

    entry.value = serde_json::to_vec(value)?;
    Ok(())
}
