/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::ephemeral_storage::EphemeralStorage;

mod backends;
pub mod ephemeral_storage;
pub mod error;
pub mod models;

#[cfg(all(not(target_arch = "wasm32"), feature = "persistent-storage"))]
pub mod persistent_storage;

pub mod storage;

#[cfg(all(not(target_arch = "wasm32"), feature = "persistent-storage"))]
pub async fn initialise_persistent_storage<P: AsRef<std::path::Path>>(
    path: P,
) -> crate::persistent_storage::PersistentStorage {
    match persistent_storage::PersistentStorage::init(path).await {
        Err(err) => panic!("failed to initialise credential storage - {err}"),
        Ok(storage) => storage,
    }
}

pub fn initialise_ephemeral_storage() -> EphemeralStorage {
    ephemeral_storage::EphemeralStorage::default()
}
