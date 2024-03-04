// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use backend::*;
pub use combined::CombinedReplyStorage;
pub use key_storage::SentReplyKeys;
pub use surb_storage::ReceivedReplySurbsMap;
pub use tag_storage::UsedSenderTags;

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
    backend: T,
}

impl<T> PersistentReplyStorage<T>
where
    T: ReplyStorageBackend + Send + Sync,
{
    pub fn new(backend: T) -> Self {
        PersistentReplyStorage { backend }
    }

    pub async fn load_state_from_backend(&self) -> Result<CombinedReplyStorage, T::StorageError> {
        self.backend.load_surb_storage().await
    }

    // this will have to get enabled after merging develop
    pub async fn flush_on_shutdown(
        mut self,
        mem_state: CombinedReplyStorage,
        mut shutdown: nym_task::TaskClient,
    ) {
        use log::{debug, error, info};

        debug!("Started PersistentReplyStorage");
        if let Err(err) = self.backend.start_storage_session().await {
            error!("failed to start the storage session - {err}");
            return;
        }

        shutdown.recv().await;

        info!("PersistentReplyStorage is flushing all reply-related data to underlying storage");
        info!("you MUST NOT forcefully shutdown now or you risk data corruption!");
        if let Err(err) = self.backend.flush_surb_storage(&mem_state).await {
            error!("failed to flush our reply-related data to the persistent storage: {err}")
        } else {
            info!("Data flush is complete")
        }

        if let Err(err) = self.backend.stop_storage_session().await {
            error!("failed to properly stop the storage session - {err}. We might not be able to smoothly restore it")
        }
    }
}
