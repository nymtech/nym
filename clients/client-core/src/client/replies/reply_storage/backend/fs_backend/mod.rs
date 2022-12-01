// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::backend::fs_backend::error::StorageError;
use crate::client::replies::reply_storage::backend::fs_backend::manager::StorageManager;
use crate::client::replies::reply_storage::backend::fs_backend::models::{
    ReplySurbStorageMetadata, StoredReplyKey, StoredReplySurb, StoredSenderTag, StoredSurbSender,
};
use crate::client::replies::reply_storage::surb_storage::ReceivedReplySurbs;
use crate::client::replies::reply_storage::{
    CombinedReplyStorage, ReceivedReplySurbsMap, ReplyStorageBackend, SentReplyKeys, UsedSenderTags,
};
use async_trait::async_trait;
use log::{info, warn};
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use std::path::{Path, PathBuf};
use std::{fs, mem};
use tokio::time::Instant;

mod error;
mod manager;
mod models;

pub(crate) struct Backend {
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

        let backend = Backend {
            temporary_old_path: None,
            database_path: owned_path,
            manager: StorageManager::init(database_path).await?,
        };

        backend.manager.create_status_table().await?;
        Ok(backend)
    }

    async fn close_pool(&mut self) {
        self.manager.connection_pool.close().await;
    }

    async fn rotate(&mut self) -> Result<(), StorageError> {
        self.close_pool().await;

        let new_extension = if let Some(existing_extension) =
            self.database_path.extension().and_then(|ext| ext.to_str())
        {
            format!("{existing_extension}.{}", Self::OLD_EXTENSION)
        } else {
            Self::OLD_EXTENSION.to_string()
        };

        let mut new_path = self.database_path.clone();
        new_path.set_extension(new_extension);

        fs::rename(&self.database_path, &new_path)
            .map_err(|err| StorageError::DatabaseRenameError { source: err })?;
        self.manager = StorageManager::init(&new_path).await?;

        let old_path = mem::replace(&mut self.database_path, new_path);
        self.temporary_old_path = Some(old_path);
        Ok(())
    }

    fn remove_old(&mut self) -> Result<(), StorageError> {
        if let Some(old_path) = self.temporary_old_path.take() {
            fs::remove_file(old_path)
                .map_err(|err| StorageError::DatabaseOldFileRemoveError { source: err })
        } else {
            warn!("the old database file doesn't seem to exist!");
            Ok(())
        }
    }

    async fn start_storage_flush(&mut self) -> Result<(), StorageError> {
        Ok(self.manager.set_flush_status(true).await?)
    }

    async fn end_storage_flush(&mut self) -> Result<(), StorageError> {
        Ok(self.manager.set_flush_status(false).await?)
    }

    async fn get_stored_tags(&self) -> Result<UsedSenderTags, StorageError> {
        let stored = self.manager.get_tags().await?;

        // stop at the first instance of corruption. if even a single entry is malformed,
        // something weird has happened and we can't trust the rest of the data
        let raw = stored
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?;

        Ok(UsedSenderTags::from_raw(raw))
    }

    async fn dump_sender_tags(&self, tags: &UsedSenderTags) -> Result<(), StorageError> {
        for map_ref in tags.as_raw_iter() {
            let (recipient, tag) = map_ref.pair();
            self.manager
                .insert_tag(StoredSenderTag::new(*recipient, *tag))
                .await?;
        }
        Ok(())
    }

    async fn get_stored_reply_keys(&self) -> Result<SentReplyKeys, StorageError> {
        let stored = self.manager.get_reply_keys().await?;

        // // stop at the first instance of corruption. if even a single entry is malformed,
        // // something weird has happened and we can't trust the rest of the data
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
                .insert_reply_key(StoredReplyKey::new(digest.clone(), *key))
                .await?;
        }
        Ok(())
    }

    async fn get_stored_reply_surbs(&self) -> Result<ReceivedReplySurbsMap, StorageError> {
        let surb_senders = self.manager.get_surb_senders().await?;

        let metadata = self.get_reply_surb_storage_metadata().await?;
        let mut received_surbs = Vec::with_capacity(surb_senders.len());
        for sender in surb_senders {
            let sender_id = sender.id;
            let (sender_tag, surbs_last_received_at): (AnonymousSenderTag, Instant) =
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

        Ok(ReceivedReplySurbsMap::from_raw(
            metadata.min_reply_surb_threshold as usize,
            metadata.max_reply_surb_threshold as usize,
            received_surbs,
        ))
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
                reply_surbs.min_surb_threshold(),
            ))
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl ReplyStorageBackend for Backend {
    type StorageError = error::StorageError;

    async fn flush_surb_storage(
        &mut self,
        storage: &CombinedReplyStorage,
    ) -> Result<(), Self::StorageError> {
        // close all connections (there should be none! and rename the file to contain .old extension)
        self.rotate().await?;
        self.start_storage_flush().await?;

        self.dump_sender_tags(storage.tags_storage_ref()).await?;
        self.dump_sender_reply_keys(storage.key_storage_ref())
            .await?;
        let surbs_ref = storage.surbs_storage_ref();
        self.dump_reply_surb_storage_metadata(surbs_ref).await?;
        self.dump_reply_surbs(surbs_ref).await?;

        self.remove_old()?;
        self.end_storage_flush().await
    }

    async fn load_surb_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError> {
        let reply_keys = self.get_stored_reply_keys().await?;
        let tags = self.get_stored_tags().await?;
        let reply_surbs = self.get_stored_reply_surbs().await?;

        Ok(CombinedReplyStorage::load(reply_keys, reply_surbs, tags))
    }
}
