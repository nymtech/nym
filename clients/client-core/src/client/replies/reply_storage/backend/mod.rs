// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::{ReceivedReplySurbsMap, SentReplyKeys, UsedSenderTags};
use async_trait::async_trait;
use nymsphinx::anonymous_replies::SurbEncryptionKey;

#[cfg(target_arch = "wasm32")]
mod browser_backend;

#[cfg(all(not(target_arch = "wasm32")), feature = "fs-surb-storage")]
mod fs_backend;

// #[cfg(all(test, feature = "std"))]
// third case: node with actual filesystem

pub struct Empty {}

impl ReplyStorageBackend for Empty {
    type StorageError = ();

    // fn insert_encryption_key(&mut self, _key: SurbEncryptionKey) -> Result<(), Self::StorageError> {
    //     Ok(())
    // }
}

#[async_trait]
pub trait ReplyStorageBackend {
    type StorageError;

    // reply keys and surbs would need additional field set when data is loaded
    // so if there's some failure, we'd trash it all

    async fn flush_reply_keys_storage(&self) -> Result<(), Self::StorageError>;
    async fn flush_received_reply_surbs_storage(&self) -> Result<(), Self::StorageError>;
    async fn flush_used_tags_storage(&self) -> Result<(), Self::StorageError>;

    async fn load_reply_keys_storage(&self) -> Result<SentReplyKeys, Self::StorageError>;
    async fn load_received_reply_surbs_storage(
        &self,
    ) -> Result<ReceivedReplySurbsMap, Self::StorageError>;
    async fn load_used_tags_storage(&self) -> Result<UsedSenderTags, Self::StorageError>;
}
