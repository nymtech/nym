// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
pub mod error;

#[cfg(target_arch = "wasm32")]
pub mod storage;

#[cfg(target_arch = "wasm32")]
pub use error::ExtensionStorageError;

#[cfg(target_arch = "wasm32")]
pub use storage::ExtensionStorage;
