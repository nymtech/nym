// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::{
    CombinedReplyStorage, ReceivedReplySurbsMap, SentReplyKeys, UsedSenderTags,
};
use async_trait::async_trait;
use nymsphinx::anonymous_replies::SurbEncryptionKey;
use std::error::Error;
use thiserror::Error;

#[cfg(target_arch = "wasm32")]
pub mod browser_backend;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
pub mod fs_backend;

// #[cfg(all(test, feature = "std"))]
// third case: node with actual filesystem

#[derive(Debug, Error)]
#[error("no information provided")]
pub struct UndefinedError;

pub struct Empty {}

#[async_trait]
impl ReplyStorageBackend for Empty {
    type StorageError = UndefinedError;

    async fn flush_surb_storage(
        &mut self,
        _storage: &CombinedReplyStorage,
    ) -> Result<(), Self::StorageError> {
        Ok(())
    }

    async fn load_surb_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError> {
        todo!()
    }
}

#[async_trait]
pub trait ReplyStorageBackend {
    type StorageError: Error;

    // reply keys and surbs would need additional field set when data is loaded
    // so if there's some failure, we'd trash it all
    async fn flush_surb_storage(
        &mut self,
        storage: &CombinedReplyStorage,
    ) -> Result<(), Self::StorageError>;

    async fn load_surb_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError>;
}
