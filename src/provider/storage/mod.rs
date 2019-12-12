use crate::provider::{MESSAGE_RETRIEVAL_LIMIT, STORED_MESSAGE_FILENAME_LENGTH};
use rand::Rng;
use sfw_provider_requests::DUMMY_MESSAGE_CONTENT;
use sphinx::route::{DestinationAddressBytes, SURBIdentifier};
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

pub enum StoreError {
    ClientDoesntExistError,
    FileIOFailure,
}

impl From<std::io::Error> for StoreError {
    fn from(_: std::io::Error) -> Self {
        use StoreError::*;

        FileIOFailure
    }
}

pub struct StoreData {
    client_address: DestinationAddressBytes,
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

// TODO: replace with database
pub struct ClientStorage(());

// TODO: change it to some generic implementation to inject fs
impl ClientStorage {
    fn generate_random_file_name() -> String {
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(STORED_MESSAGE_FILENAME_LENGTH)
            .collect::<String>()
    }

    fn dummy_message() -> Vec<u8> {
        // TODO: should it be padded to constant length?
        DUMMY_MESSAGE_CONTENT.to_vec()
    }

    pub fn store_processed_data(store_data: StoreData, store_dir: &Path) -> io::Result<()> {
        let client_dir_name = hex::encode(store_data.client_address);
        let full_store_dir = store_dir.join(client_dir_name);
        let full_store_path = full_store_dir.join(ClientStorage::generate_random_file_name());
        println!(
            "going to store: {:?} in file: {:?}",
            store_data.message, full_store_path
        );

        // TODO: what to do with surbIDs??

        // TODO: this should be called when client sends 'register' request!
        std::fs::create_dir_all(full_store_dir)?;

        // we can use normal io here, no need for tokio as it's all happening in one thread per connection
        let mut file = File::create(full_store_path)?;
        file.write_all(store_data.message.as_ref())?;

        Ok(())
    }

    pub fn retrieve_client_files(
        client_address: DestinationAddressBytes,
        store_dir: &Path,
    ) -> Result<Vec<Vec<u8>>, StoreError> {
        let client_dir_name = hex::encode(client_address);
        let full_store_dir = store_dir.join(client_dir_name);

        println!("going to lookup: {:?}!", full_store_dir);
        if !full_store_dir.exists() {
            return Err(StoreError::ClientDoesntExistError);
        }

        let msgs: Vec<_> = std::fs::read_dir(full_store_dir)?
            .map(|entry| entry.unwrap())
            .filter(|entry| {
                let is_file = entry.metadata().unwrap().is_file();
                if !is_file {
                    eprintln!(
                        "potentially corrupted client inbox! - found a non-file - {:?}",
                        entry.path()
                    );
                }
                is_file
            })
            .map(|entry| {
                let content = std::fs::read(entry.path()).unwrap();
                ClientStorage::delete_file(entry.path());
                content
            }) // TODO: THIS MAP IS UNSAFE (BOTH READING AND DELETING)!!
            .chain(std::iter::repeat(ClientStorage::dummy_message()))
            .take(MESSAGE_RETRIEVAL_LIMIT)
            .collect();

        println!("retrieved the following data: {:?}", msgs);

        Ok(msgs)
    }

    // TODO: THIS NEEDS A LOCKING MECHANISM!!! (or a db layer on top - basically 'ClientStorage' on steroids)
    // TODO 2: This should only be called AFTER we sent the reply. Because if client's connection failed after sending request
    // the messages would be deleted but he wouldn't have received them
    fn delete_file(path: PathBuf) {
        println!("Here {:?} will be deleted!", path);
        std::fs::remove_file(path); // another argument for db layer -> remove_file is NOT guaranteed to immediately get rid of the file
    }
}
