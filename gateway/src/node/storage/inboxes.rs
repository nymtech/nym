// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

        let client_dir_name = client_address.to_base58_string();
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

        let client_dir_name = store_data.client_address.to_base58_string();
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

        let client_dir_name = client_address.to_base58_string();
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
