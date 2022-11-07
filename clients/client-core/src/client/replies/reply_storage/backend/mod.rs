// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nymsphinx::anonymous_replies::SurbEncryptionKey;

#[cfg(target_arch = "wasm32")]
mod browser_backend;

#[cfg(not(target_arch = "wasm32"))]
mod fs_backend;

// implemented via trait as implementations are going to vary wildly between wasm and non-wasm targets
// TODO: might need to be transformed into an async-trait. not sure yet
trait ReplyStorageBackend {
    type StorageError;

    fn insert_encryption_key(&mut self, key: SurbEncryptionKey) -> Result<(), Self::StorageError>;
}
