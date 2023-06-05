/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::ephemeral_storage::EphemeralStorage;

#[cfg(not(target_arch = "wasm32"))]
use crate::persistent_storage::PersistentStorage;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

mod backends;
pub mod ephemeral_storage;
pub mod error;
pub mod models;
#[cfg(not(target_arch = "wasm32"))]
pub mod persistent_storage;
pub mod storage;

#[cfg(not(target_arch = "wasm32"))]
pub async fn initialise_persistent_storage<P: AsRef<Path>>(path: P) -> PersistentStorage {
    match persistent_storage::PersistentStorage::init(path).await {
        Err(err) => panic!("failed to initialise credential storage - {err}"),
        Ok(storage) => storage,
    }
}

pub fn initialise_ephemeral_storage() -> EphemeralStorage {
    ephemeral_storage::EphemeralStorage::default()
}
