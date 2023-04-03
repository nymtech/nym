/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::ephemeral_storage::EphemeralStorage;
use crate::persistent_storage::PersistentStorage;
use std::path::PathBuf;

mod backends;
pub mod ephemeral_storage;
pub mod error;
mod models;
pub mod persistent_storage;
pub mod storage;

pub async fn initialise_persistent_storage(path: PathBuf) -> PersistentStorage {
    match persistent_storage::PersistentStorage::init(path).await {
        Err(err) => panic!("failed to initialise credential storage - {err}"),
        Ok(storage) => storage,
    }
}

pub async fn initialise_ephemeral_storage() -> EphemeralStorage {
    ephemeral_storage::EphemeralStorage::default()
}
