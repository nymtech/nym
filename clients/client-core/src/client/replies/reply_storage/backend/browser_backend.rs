// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::backend::Empty;
use crate::client::replies::reply_storage::{CombinedReplyStorage, ReplyStorageBackend};
use async_trait::async_trait;

use std::path::PathBuf;

// well, right now we don't have the browser storage : (
// so we keep everything in memory
#[derive(Debug)]
pub struct Backend {
    empty: Empty,
}

impl Backend {
    pub fn new(min_surb_threshold: usize, max_surb_threshold: usize) -> Self {
        Backend {
            empty: Empty {
                min_surb_threshold,
                max_surb_threshold,
            },
        }
    }
}

#[async_trait]
impl ReplyStorageBackend for Backend {
    type StorageError = <Empty as ReplyStorageBackend>::StorageError;

    async fn new(
        debug_config: &crate::config::DebugConfig,
        _db_path: Option<PathBuf>,
    ) -> Result<Self, Self::StorageError> {
        Ok(Backend {
            empty: Empty {
                min_surb_threshold: debug_config.minimum_reply_surb_storage_threshold,
                max_surb_threshold: debug_config.maximum_reply_surb_storage_threshold,
            },
        })
    }

    async fn flush_surb_storage(
        &mut self,
        storage: &CombinedReplyStorage,
    ) -> Result<(), Self::StorageError> {
        self.empty.flush_surb_storage(storage).await
    }

    async fn init_fresh(&mut self, fresh: &CombinedReplyStorage) -> Result<(), Self::StorageError> {
        self.empty.init_fresh(fresh).await
    }

    async fn load_surb_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError> {
        self.empty.load_surb_storage().await
    }

    fn get_inactive_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError> {
        self.empty.get_inactive_storage()
    }
}
