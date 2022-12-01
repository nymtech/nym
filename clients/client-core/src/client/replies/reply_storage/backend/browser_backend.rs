// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::backend::Empty;
use crate::client::replies::reply_storage::{
    CombinedReplyStorage, ReceivedReplySurbsMap, ReplyStorageBackend, SentReplyKeys, UsedSenderTags,
};
use async_trait::async_trait;

// well, right now we don't have the browser storage : (
// so we keep everything in memory
pub struct Backend {
    empty: Empty,
}

#[async_trait]
impl ReplyStorageBackend for Backend {
    async fn flush_surb_storage(
        &self,
        storage: &CombinedReplyStorage,
    ) -> Result<(), Self::StorageError> {
        self.empty.flush_surb_storage(storage)
    }

    async fn load_surb_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError> {
        self.empty.load_surb_storage()
    }
}
