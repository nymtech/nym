// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::client::replies::reply_storage::combined::CombinedReplyStorage;
pub use crate::client::replies::reply_storage::key_storage::SentReplyKeys;
pub use crate::client::replies::reply_storage::surb_storage::ReceivedReplySurbsMap;
pub use crate::client::replies::reply_storage::tag_storage::UsedSenderTags;
pub use backend::*;

use log::{debug, error, info, warn};

mod backend;
mod combined;
mod key_storage;
mod surb_storage;
mod tag_storage;

// only really exists to get information about shutdown and save data to the backing storage
pub struct PersistentReplyStorage<T = backend::Empty>
where
    T: ReplyStorageBackend,
{
    combined_storage: CombinedReplyStorage,
    backend: T,
}

impl<T> PersistentReplyStorage<T>
where
    T: ReplyStorageBackend,
{
    pub fn new(combined_storage: CombinedReplyStorage, backend: T) -> Self {
        PersistentReplyStorage {
            combined_storage,
            backend,
        }
    }

    pub async fn flush_on_shutdown(mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started PersistentReplyStorage");
        shutdown.recv().await;

        info!("PersistentReplyStorage is flushing all reply-related data to underlying storage");
        warn!("you MUST NOT forcefully shutdown now or you risk data corruption!");
        if let Err(err) = self
            .backend
            .flush_surb_storage(&self.combined_storage)
            .await
        {
            error!("failed to flush our reply-related data to the persistent storage: {err}")
        } else {
            info!("Data flush is complete")
        }
    }
}
