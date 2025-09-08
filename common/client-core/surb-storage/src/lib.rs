// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use backend::*;
pub use combined::CombinedReplyStorage;
pub use key_storage::SentReplyKeys;
pub use surb_storage::{ReceivedReplySurb, ReceivedReplySurbsMap, RetrievedReplySurb};
pub use tag_storage::UsedSenderTags;
use time::OffsetDateTime;

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

    pub async fn load_state_from_backend(
        &self,
        surb_freshness_cutoff: OffsetDateTime,
    ) -> Result<CombinedReplyStorage, T::StorageError> {
        self.backend.load_surb_storage(surb_freshness_cutoff).await
    }

    pub async fn flush_on_shutdown(
        mut self,
        mem_state: CombinedReplyStorage,
        shutdown: nym_task::ShutdownToken,
    ) {
        use tracing::{debug, error, info};

        debug!("Started PersistentReplyStorage");
        if let Err(err) = self.backend.start_storage_session().await {
            error!("failed to start the storage session - {err}");
            return;
        }

        shutdown.cancelled().await;

        info!("PersistentReplyStorage is flushing all reply-related data to underlying storage");
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
