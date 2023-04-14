// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::client::key_manager::KeyManager;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::config;
use rand::rngs::OsRng;
use wasm_utils::console_log;

pub(crate) fn setup_new_key_manager() -> KeyManager {
    let mut rng = OsRng;
    console_log!("generated new set of keys");
    KeyManager::new(&mut rng)
}

// don't get too excited about the name, under the hood it's just a big fat placeholder
// with no persistence
pub(crate) fn setup_reply_surb_storage_backend(
    config: config::ReplySurbs,
) -> browser_backend::Backend {
    browser_backend::Backend::new(
        config.minimum_reply_surb_storage_threshold,
        config.maximum_reply_surb_storage_threshold,
    )
}
