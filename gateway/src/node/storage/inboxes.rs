// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::storage::models::StoredMessage;
use futures::lock::Mutex;
use futures::StreamExt;
use gateway_requests::DUMMY_MESSAGE_CONTENT;
use log::*;
use nymsphinx::DestinationAddressBytes;
use rand::Rng;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReadDirStream;

#[derive(Clone)]
pub(crate) struct InboxManager {
    connection_pool: sqlx::SqlitePool,
    /// Maximum number of messages that can be obtained from the database per operation.
    /// It is used to prevent memory overflows in the case of client receiving a lot of data while
    /// offline and then loading it all at once when he comes back online.
    retrieval_limit: i64,
}

impl InboxManager {
    /// Creates new instance of the `InboxManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool, retrieval_limit: i64) -> Self {
        InboxManager {
            connection_pool,
            retrieval_limit,
        }
    }

    /// Inserts new message to the storage for an offline client for future retrieval.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client
    /// * `content`: raw content of the message to store.
    pub(crate) async fn insert_message(
        &self,
        client_address_bs58: &str,
        content: Vec<u8>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO message_store(client_address_bs58, content) VALUES (?, ?)",
            client_address_bs58,
            content,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Retrieves messages stored for the particular client specified by the provided address.
    ///
    /// It also respects the specified retrieval limit. If there are more messages stored than allowed
    /// by the limit, it returns id of the last message retrieved to indicate start of the next query.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client
    /// * `start_after`: optional starting id of the messages to grab
    ///
    /// returns the retrieved messages alongside optional id of the last message retrieved if
    /// there are more messages to retrieve.
    pub(crate) async fn get_messages(
        &self,
        client_address_bs58: &str,
        start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), sqlx::Error> {
        // get 1 additional message to check whether there will be more to grab
        // next time
        let limit = self.retrieval_limit + 1;
        let mut res = if let Some(start_after) = start_after {
            sqlx::query_as!(
                StoredMessage,
                r#"
                    SELECT * FROM message_store 
                    WHERE client_address_bs58 = ? AND id > ?
                    ORDER BY id ASC
                    LIMIT ?;
                "#,
                start_after,
                client_address_bs58,
                limit
            )
            .fetch_all(&self.connection_pool)
            .await?
        } else {
            sqlx::query_as!(
                StoredMessage,
                r#"
                    SELECT * FROM message_store 
                    WHERE client_address_bs58 = ?
                    ORDER BY id ASC
                    LIMIT ?;
                "#,
                client_address_bs58,
                limit
            )
            .fetch_all(&self.connection_pool)
            .await?
        };

        if res.len() > self.retrieval_limit as usize {
            res.truncate(self.retrieval_limit as usize);
            // assuming retrieval_limit > 0, unwrap will not fail
            let start_after = res.last().unwrap().id;
            Ok((res, Some(start_after)))
            //
        } else {
            Ok((res, None))
        }
    }

    /// Removes message with the specified id
    ///
    /// # Arguments
    ///
    /// * `id`: id of the message to remove
    pub(crate) async fn remove_message(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM message_store WHERE id = ?", id)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }
}

fn dummy_message() -> ClientFile {
    ClientFile {
        content: DUMMY_MESSAGE_CONTENT.to_vec(),
        path: Default::default(),
    }
}

#[derive(Clone, Debug)]
pub struct ClientFile {
    content: Vec<u8>,
    path: PathBuf,
}

impl ClientFile {
    fn new(content: Vec<u8>, path: PathBuf) -> Self {
        ClientFile { content, path }
    }

    pub(crate) fn into_tuple(self) -> (Vec<u8>, PathBuf) {
        (self.content, self.path)
    }
}

pub struct StoreData {
    client_address: DestinationAddressBytes,
    message: Vec<u8>,
}

impl StoreData {
    pub(crate) fn new(client_address: DestinationAddressBytes, message: Vec<u8>) -> Self {
        StoreData {
            client_address,
            message,
        }
    }
}

// TODO: replace with proper database...
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
#[derive(Clone, Debug)]
pub struct ClientStorage {
    inner: Arc<Mutex<ClientStorageInner>>,
}

// even though the data inside is extremely cheap to copy, we have to have a single mutex,
// so might as well store the data behind it
pub struct ClientStorageInner {
    // basically part of rate limiting which does not exist anymore
    #[allow(dead_code)]
    message_retrieval_limit: usize,
    filename_length: u16,
    main_store_path_dir: PathBuf,
}

// TODO: change it to some generic implementation to inject fs (or even better - proper database)
impl ClientStorage {
    pub(crate) fn new(message_limit: usize, filename_len: u16, main_store_dir: PathBuf) -> Self {
        ClientStorage {
            inner: Arc::new(Mutex::new(ClientStorageInner {
                message_retrieval_limit: message_limit,
                filename_length: filename_len,
                main_store_path_dir: main_store_dir,
            })),
        }
    }

    pub(crate) async fn create_storage_dir(
        &self,
        client_address: DestinationAddressBytes,
    ) -> io::Result<()> {
        let inner_data = self.inner.lock().await;

        let client_dir_name = client_address.as_base58_string();
        let full_store_dir = inner_data.main_store_path_dir.join(client_dir_name);
        fs::create_dir_all(full_store_dir).await
    }

    pub(crate) fn generate_random_file_name(length: usize) -> String {
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(length)
            .collect::<String>()
    }

    pub(crate) async fn store_processed_data(&self, store_data: StoreData) -> io::Result<()> {
        let inner_data = self.inner.lock().await;

        let client_dir_name = store_data.client_address.as_base58_string();
        let full_store_dir = inner_data.main_store_path_dir.join(client_dir_name);
        let full_store_path = full_store_dir.join(Self::generate_random_file_name(
            inner_data.filename_length as usize,
        ));
        trace!(
            "going to store: {:?} in file: {:?}",
            store_data.message,
            full_store_path
        );

        let mut file = File::create(full_store_path).await?;
        file.write_all(store_data.message.as_ref()).await
    }

    pub(crate) async fn retrieve_all_client_messages(
        &self,
        client_address: DestinationAddressBytes,
    ) -> io::Result<Vec<ClientFile>> {
        let inner_data = self.inner.lock().await;

        let client_dir_name = client_address.as_base58_string();
        let full_store_dir = inner_data.main_store_path_dir.join(client_dir_name);

        trace!("going to lookup: {:?}!", full_store_dir);
        if !full_store_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Target client does not exist",
            ));
        }

        let mut msgs = Vec::new();
        let mut read_dir = ReadDirStream::new(fs::read_dir(full_store_dir).await?);
        while let Some(dir_entry) = read_dir.next().await {
            if let Ok(dir_entry) = dir_entry {
                if !Self::is_valid_file(&dir_entry).await {
                    continue;
                }
                let client_file =
                    ClientFile::new(fs::read(dir_entry.path()).await?, dir_entry.path());
                msgs.push(client_file)
            }
        }
        Ok(msgs)
    }

    async fn is_valid_file(entry: &fs::DirEntry) -> bool {
        let metadata = match entry.metadata().await {
            Ok(meta) => meta,
            Err(e) => {
                error!(
                    "potentially corrupted client inbox! ({:?} - failed to read its metadata - {:?}",
                    entry.path(),
                    e,
                );
                return false;
            }
        };

        let is_file = metadata.is_file();
        if !is_file {
            error!(
                "potentially corrupted client inbox! - found a non-file - {:?}",
                entry.path()
            );
        }

        is_file
    }

    pub(crate) async fn delete_files(&self, file_paths: Vec<PathBuf>) -> io::Result<()> {
        let dummy_message = dummy_message();
        let _guard = self.inner.lock().await;

        for file_path in file_paths {
            if file_path == dummy_message.path {
                continue;
            }
            if let Err(e) = fs::remove_file(file_path).await {
                error!("Failed to delete client message! - {:?}", e)
            }
        }
        Ok(())
    }
}
