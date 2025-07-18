// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backend::fs_backend::{
        manager::StorageManager,
        models::{ReplySurbStorageMetadata, StoredReplyKey, StoredReplySurb, StoredSurbSender},
    },
    surb_storage::ReceivedReplySurbs,
    CombinedReplyStorage, ReceivedReplySurbsMap, ReplyStorageBackend, SentReplyKeys,
};
use async_trait::async_trait;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use tracing::{error, info, warn};

pub use self::error::StorageError;

mod error;
mod manager;
mod models;

#[derive(Clone, Debug)]
pub struct Backend {
    temporary_old_path: Option<PathBuf>,
    database_path: PathBuf,
    manager: StorageManager,
}

impl Backend {
    const OLD_EXTENSION: &'static str = "old";

    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        let owned_path: PathBuf = database_path.as_ref().into();
        if owned_path.file_name().is_none() {
            return Err(StorageError::DatabasePathWithoutFilename {
                provided_path: owned_path,
            });
        }

        let manager = StorageManager::init(database_path, true).await?;
        match manager.create_status_table().await {
            Ok(()) => Ok(Backend {
                temporary_old_path: None,
                database_path: owned_path,
                manager,
            }),
            Err(err) => {
                manager.close_pool().await;
                Err(err.into())
            }
        }
    }

    pub async fn try_load<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        let owned_path: PathBuf = database_path.as_ref().into();
        if owned_path.file_name().is_none() {
            return Err(StorageError::DatabasePathWithoutFilename {
                provided_path: owned_path,
            });
        }

        let manager = StorageManager::init(database_path, false).await?;
        match Self::try_load_inner(&manager).await {
            Ok(()) => Ok(Backend {
                temporary_old_path: None,
                database_path: owned_path,
                manager,
            }),
            Err(e) => {
                manager.close_pool().await;
                Err(e)
            }
        }
    }

    /// Gracefully close sqlite connection pool and drop backend.
    pub async fn shutdown(self) {
        self.manager.close_pool().await
    }

    async fn try_load_inner(manager: &StorageManager) -> Result<(), StorageError> {
        // the database flush wasn't fully finished and thus the data is in inconsistent state
        // (we don't really know what's properly saved or what's not)
        if manager.get_flush_status().await? {
            return Err(StorageError::IncompleteDataFlush);
        }

        let last_flush = manager.get_previous_flush_time().await?;
        if last_flush == OffsetDateTime::UNIX_EPOCH {
            // either this client has been running since 1970 or the flush failed
            return Err(StorageError::IncompleteDataFlush);
        }

        // the process has gone down without full graceful shutdown,
        // meaning the database doesn't contain valid data anymore
        // so we have to purge it
        if manager.get_client_in_use_status().await? {
            error!("the client hasn't undergone through graceful shutdown the last time it's gone down - we can't trust its reply surbs or stored encryption keys. They shall get purged");
            manager.delete_all_reply_surb_data().await?;
            manager.delete_all_reply_keys().await?;
        }

        if let Err(err) = manager.get_reply_surb_storage_metadata().await {
            // we can't recover here, we HAVE TO initialise fresh (because we don't know correct starting metadata)
            error!("it seems the client has been shutdown gracefully - we're missing valid surb data dump. the existing database cannot be used");
            return Err(err.into());
        }

        // in theory clients can use our reply surbs whenever they want, even a year in the future
        // (assuming no key rotation has happened)
        // but the way it's currently coded, everyone will purge old data
        let since_last_flush = OffsetDateTime::now_utc() - last_flush;
        let days = since_last_flush.whole_days();
        let hours = since_last_flush.whole_hours() % 24;

        if days > 0 {
            info!("it's been over {days} days and {hours} hours since we last used our data store. our reply surbs are already outdated - we're going to purge them now.");
            manager.delete_all_reply_surb_data().await?;
        }

        if days > 1 {
            info!("it's been over {days} days and {hours} hours since we last used our data store. our reply keys are already outdated - we're going to purge them now.");
            manager.delete_all_reply_keys().await?;
        }

        Ok(())
    }

    async fn rotate(&mut self) -> Result<(), StorageError> {
        self.manager.close_pool().await;

        let new_extension = if let Some(existing_extension) =
            self.database_path.extension().and_then(|ext| ext.to_str())
        {
            format!("{existing_extension}.{}", Self::OLD_EXTENSION)
        } else {
            Self::OLD_EXTENSION.to_string()
        };

        let mut temp_old = self.database_path.clone();
        temp_old.set_extension(new_extension);

        tokio::fs::rename(&self.database_path, &temp_old)
            .await
            .map_err(|err| StorageError::DatabaseRenameError { source: err })?;
        self.manager = StorageManager::init(&self.database_path, true).await?;
        self.manager.create_status_table().await?;

        self.temporary_old_path = Some(temp_old);
        Ok(())
    }

    async fn remove_old(&mut self) -> Result<(), StorageError> {
        if let Some(old_path) = self.temporary_old_path.take() {
            tokio::fs::remove_file(old_path)
                .await
                .map_err(|err| StorageError::DatabaseOldFileRemoveError { source: err })
        } else {
            warn!("the old database file doesn't seem to exist!");
            Ok(())
        }
    }

    async fn start_storage_flush(&self) -> Result<(), StorageError> {
        Ok(self.manager.set_flush_status(true).await?)
    }

    async fn end_storage_flush(&self) -> Result<(), StorageError> {
        self.manager
            .set_previous_flush(OffsetDateTime::now_utc())
            .await?;
        Ok(self.manager.set_flush_status(false).await?)
    }

    async fn start_client_use(&self) -> Result<(), StorageError> {
        Ok(self.manager.set_client_in_use_status(true).await?)
    }

    async fn stop_client_use(&self) -> Result<(), StorageError> {
        Ok(self.manager.set_client_in_use_status(false).await?)
    }

    async fn get_stored_reply_keys(&self) -> Result<SentReplyKeys, StorageError> {
        let stored = self.manager.get_reply_keys().await?;

        // stop at the first instance of corruption. if even a single entry is malformed,
        // something weird has happened and we can't trust the rest of the data
        let raw = stored
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?;

        Ok(SentReplyKeys::from_raw(raw))
    }

    async fn dump_sender_reply_keys(&self, reply_keys: &SentReplyKeys) -> Result<(), StorageError> {
        for map_ref in reply_keys.as_raw_iter() {
            let (digest, key) = map_ref.pair();
            self.manager
                .insert_reply_key(StoredReplyKey::new(*digest, *key))
                .await?;
        }
        Ok(())
    }

    async fn get_stored_reply_surbs(
        &self,
        surb_freshness_cutoff: OffsetDateTime,
    ) -> Result<ReceivedReplySurbsMap, StorageError> {
        let surb_senders = self.manager.get_surb_senders().await?;

        let metadata = self.get_reply_surb_storage_metadata().await?;
        let mut received_surbs = Vec::with_capacity(surb_senders.len());
        for sender in surb_senders {
            let sender_id = sender.id;
            let (sender_tag, surbs_last_received_at): (AnonymousSenderTag, OffsetDateTime) =
                sender.try_into()?;
            let stored_surbs = self
                .manager
                .get_reply_surbs(sender_id)
                .await?
                .into_iter()
                .map(|raw| raw.try_into())
                .collect::<Result<_, _>>()?;

            received_surbs.push((
                sender_tag,
                ReceivedReplySurbs::new_retrieved(stored_surbs, surbs_last_received_at),
            ))
        }

        let received_surbs = ReceivedReplySurbsMap::from_raw(
            metadata.min_reply_surb_threshold as usize,
            metadata.max_reply_surb_threshold as usize,
            received_surbs,
        );
        received_surbs.drop_stale_loaded_surbs(surb_freshness_cutoff);
        Ok(received_surbs)
    }

    async fn dump_reply_surbs(
        &self,
        reply_surbs: &ReceivedReplySurbsMap,
    ) -> Result<(), StorageError> {
        for map_ref in reply_surbs.as_raw_iter() {
            let (tag, received_surbs) = map_ref.pair();
            let sender_id = self
                .manager
                .insert_surb_sender(StoredSurbSender::new(
                    *tag,
                    received_surbs.surbs_last_received_at(),
                ))
                .await?;

            for reply_surb in received_surbs.surbs_ref() {
                self.manager
                    .insert_reply_surb(StoredReplySurb::new(sender_id, reply_surb))
                    .await?
            }

            // TODO: should we also retain the stale ones?
            if received_surbs.possibly_stale_left() != 0 {
                warn!(
                    "dropping {} possibly stale surbs for {tag}",
                    received_surbs.possibly_stale_left()
                );
            }
        }
        Ok(())
    }

    async fn get_reply_surb_storage_metadata(
        &self,
    ) -> Result<ReplySurbStorageMetadata, StorageError> {
        self.manager
            .get_reply_surb_storage_metadata()
            .await
            .map_err(Into::into)
    }

    async fn dump_reply_surb_storage_metadata(
        &self,
        reply_surbs: &ReceivedReplySurbsMap,
    ) -> Result<(), StorageError> {
        self.manager
            .insert_reply_surb_storage_metadata(ReplySurbStorageMetadata::new(
                reply_surbs.min_surb_threshold(),
                reply_surbs.max_surb_threshold(),
            ))
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl ReplyStorageBackend for Backend {
    type StorageError = error::StorageError;

    async fn start_storage_session(&self) -> Result<(), Self::StorageError> {
        self.start_client_use().await
    }

    async fn flush_surb_storage(
        &mut self,
        storage: &CombinedReplyStorage,
    ) -> Result<(), Self::StorageError> {
        // close all connections (there should be none! and rename the file to contain .old extension)
        self.rotate().await?;
        self.start_storage_flush().await?;

        self.dump_sender_reply_keys(storage.key_storage_ref())
            .await?;
        let surbs_ref = storage.surbs_storage_ref();
        self.dump_reply_surb_storage_metadata(surbs_ref).await?;
        self.dump_reply_surbs(surbs_ref).await?;

        self.remove_old().await?;
        self.end_storage_flush().await
    }

    async fn init_fresh(&mut self, fresh: &CombinedReplyStorage) -> Result<(), Self::StorageError> {
        // for now nothing more to do apart from dumping the metadata
        self.dump_reply_surb_storage_metadata(fresh.surbs_storage_ref())
            .await
    }

    async fn load_surb_storage(
        &self,
        surb_freshness_cutoff: OffsetDateTime,
    ) -> Result<CombinedReplyStorage, Self::StorageError> {
        let reply_keys = self.get_stored_reply_keys().await?;
        let reply_surbs = self.get_stored_reply_surbs(surb_freshness_cutoff).await?;

        Ok(CombinedReplyStorage::load(reply_keys, reply_surbs))
    }

    async fn stop_storage_session(self) -> Result<(), Self::StorageError> {
        self.stop_client_use().await
    }
}
