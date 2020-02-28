use futures::lock::Mutex;
use futures::StreamExt;
use log::*;
use rand::Rng;
use sfw_provider_requests::DUMMY_MESSAGE_CONTENT;
use sphinx::route::{DestinationAddressBytes, SURBIdentifier};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::File;
use tokio::prelude::*;

const DUMMY_MESSAGE: ClientFile = ClientFile {
    content: DUMMY_MESSAGE_CONTENT.to_vec(),
    path: Default::default(),
};

#[derive(Clone)]
pub struct ClientFile {
    content: Vec<u8>,
    path: PathBuf,
}

impl ClientFile {
    fn new(content: Vec<u8>, path: PathBuf) -> Self {
        ClientFile { content, path }
    }
}

pub struct StoreData {
    client_address: DestinationAddressBytes,
    #[allow(dead_code)]
    client_surb_id: SURBIdentifier,
    message: Vec<u8>,
}

impl StoreData {
    pub(crate) fn new(
        client_address: DestinationAddressBytes,
        client_surb_id: SURBIdentifier,
        message: Vec<u8>,
    ) -> Self {
        StoreData {
            client_address,
            client_surb_id,
            message,
        }
    }
}

// TODO: replace with proper database...
pub struct ClientStorage {
    inner: Arc<Mutex<ClientStorageInner>>,
}

// even though the data inside is extremely cheap to copy, we have to have a single mutex,
// so might as well store the data behind it
pub struct ClientStorageInner {
    message_retrieval_limit: usize,
    filename_length: u16,
    main_store_path_dir: PathBuf,
}

// TODO: change it to some generic implementation to inject fs (or even better - proper database)
impl ClientStorage {
    pub(crate) fn generate_random_file_name(length: usize) -> String {
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(length)
            .collect::<String>()
    }

    pub(crate) async fn store_processed_data(&self, store_data: StoreData) -> io::Result<()> {
        let inner_data = self.inner.lock().await;

        // TODO: to replace with store_data.to_b58_string() once PR on sphinx is merged
        let client_dir_name = bs58::encode(store_data.client_address).into_string();
        let full_store_dir = inner_data.main_store_path_dir.join(client_dir_name);
        let full_store_path = full_store_dir.join(Self::generate_random_file_name(
            inner_data.filename_length as usize,
        ));
        debug!(
            "going to store: {:?} in file: {:?}",
            store_data.message, full_store_path
        );

        // TODO: what to do with surbIDs??

        let mut file = File::create(full_store_path).await?;
        file.write_all(store_data.message.as_ref()).await
    }

    pub(crate) async fn retrieve_client_files(
        &self,
        client_address: DestinationAddressBytes,
    ) -> io::Result<Vec<ClientFile>> {
        let inner_data = self.inner.lock().await;

        // TODO: to replace with store_data.to_b58_string() once PR on sphinx is merged
        let client_dir_name = bs58::encode(client_address).into_string();
        let full_store_dir = inner_data.main_store_path_dir.join(client_dir_name);

        trace!("going to lookup: {:?}!", full_store_dir);
        if !full_store_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Target client does not exist",
            ));
        }

        let mut msgs = Vec::new();
        let read_dir = fs::read_dir(full_store_dir).await?;

        while let Some(dir_entry) = read_dir.next().await {
            if let Ok(dir_entry) = dir_entry {
                if !Self::is_valid_file(&dir_entry).await {
                    continue;
                }
                // Do not delete the file itself here!
                // Only do it after client has received it
                let client_file =
                    ClientFile::new(fs::read(dir_entry.path()).await?, dir_entry.path());
                msgs.push(client_file)
            }
            if msgs.len() == inner_data.message_retrieval_limit {
                break;
            }
        }

        // make sure we always return as many messages as we need
        if msgs.len() != inner_data.message_retrieval_limit as usize {
            msgs = msgs
                .into_iter()
                .chain(std::iter::repeat(DUMMY_MESSAGE))
                .take(inner_data.message_retrieval_limit)
                .collect();
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

    fn delete_files(&self, files: Vec<ClientFile>) -> io::Result<()> {
        unimplemented!()
    }

    // TODO: This should only be called AFTER we sent the reply. Because if client's connection failed after sending request
    // the messages would be deleted but he wouldn't have received them
    fn delete_file(&self, path: PathBuf) -> io::Result<()> {
        trace!("Here {:?} will be deleted!", path);
        std::fs::remove_file(path) // another argument for db layer -> remove_file is NOT guaranteed to immediately get rid of the file
    }
}
