use crate::provider::storage::{ClientStorage, StoreError};
use crate::provider::ClientLedger;
use curve25519_dalek::scalar::Scalar;
use futures::lock::Mutex as FMutex;
use hmac::{Hmac, Mac};
use sfw_provider_requests::requests::{
    ProviderRequestError, ProviderRequests, PullRequest,
    RegisterRequest,
};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse, RegisterResponse};
use sha2::Sha256;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sfw_provider_requests::AuthToken;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub enum ClientProcessingError {
    ClientDoesntExistError,
    StoreError,
    InvalidRequest,
    WrongToken,
    IOError,
}

impl From<ProviderRequestError> for ClientProcessingError {
    fn from(_: ProviderRequestError) -> Self {
        use ClientProcessingError::*;

        InvalidRequest
    }
}

impl From<StoreError> for ClientProcessingError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ClientDoesntExistError => ClientProcessingError::ClientDoesntExistError,
            _ => ClientProcessingError::StoreError,
        }
    }
}

impl From<io::Error> for ClientProcessingError {
    fn from(_: io::Error) -> Self {
        use ClientProcessingError::*;

        IOError
    }
}

#[derive(Debug)]
pub(crate) struct ClientProcessingData {
    store_dir: PathBuf,
    registered_clients_ledger: ClientLedger,
    secret_key: Scalar,
}

impl ClientProcessingData {
    pub(crate) fn new(
        store_dir: PathBuf,
        registered_clients_ledger: ClientLedger,
        secret_key: Scalar,
    ) -> Self {
        ClientProcessingData {
            store_dir,
            registered_clients_ledger,
            secret_key,
        }
    }

    pub(crate) fn add_arc_futures_mutex(self) -> Arc<FMutex<Self>> {
        Arc::new(FMutex::new(self))
    }
}

pub(crate) struct ClientRequestProcessor(());

impl ClientRequestProcessor {
    pub(crate) async fn process_client_request(
        data: &[u8],
        processing_data: Arc<FMutex<ClientProcessingData>>,
    ) -> Result<Vec<u8>, ClientProcessingError> {
        let client_request = ProviderRequests::from_bytes(&data)?;
        println!("Received the following request: {:?}", client_request);
        match client_request {
            ProviderRequests::Register(req) => Ok(ClientRequestProcessor::register_new_client(
                req,
                processing_data,
            ).await?.to_bytes()),
            ProviderRequests::PullMessages(req) => Ok(ClientRequestProcessor::process_pull_messages_request(
                req,
                processing_data,
            ).await?.to_bytes())
        }
    }

    async fn process_pull_messages_request(
        req: PullRequest,
        processing_data: Arc<FMutex<ClientProcessingData>>,
    ) -> Result<PullResponse, ClientProcessingError> {
        // TODO: this lock is completely unnecessary as we're only reading the data.
        // Wait for https://github.com/nymtech/nym-sfw-provider/issues/19 to resolve.
        let unlocked = processing_data.lock().await;

        println!("Processing pull!");
        if unlocked.registered_clients_ledger.has_token(req.auth_token) {
            let store_dir_clone = unlocked.store_dir.clone();
            // drop the mutex so that we could do IO without blocking others wanting to get the lock
            drop(unlocked);
            let retrieved_messages =
                ClientStorage::retrieve_client_files(req.destination_address, store_dir_clone.as_path())?;
            Ok(PullResponse::new(retrieved_messages))
        } else {
            Err(ClientProcessingError::WrongToken)
        }
    }

    async fn register_new_client(
        req: RegisterRequest,
        processing_data: Arc<FMutex<ClientProcessingData>>,
    ) -> Result<RegisterResponse, ClientProcessingError> {
        println!("Processing register new client request!");
        let mut unlocked = processing_data.lock().await;

        let auth_token = ClientRequestProcessor::generate_new_auth_token(
            req.destination_address.to_vec(),
            unlocked.secret_key,
        );
        if !unlocked.registered_clients_ledger.has_token(auth_token) {
            unlocked.registered_clients_ledger.insert_token(auth_token, req.destination_address);
            ClientRequestProcessor::create_storage_dir(req.destination_address, unlocked.store_dir.as_path())?;
        }
        Ok(RegisterResponse::new(auth_token))
    }

    fn create_storage_dir(
        client_address: sphinx::route::DestinationAddressBytes,
        store_dir: &Path,
    ) -> io::Result<()> {
        let client_dir_name = hex::encode(client_address);
        let full_store_dir = store_dir.join(client_dir_name);
        std::fs::create_dir_all(full_store_dir)
    }

    fn generate_new_auth_token(data: Vec<u8>, key: Scalar) -> AuthToken {
        let mut auth_token_raw =
            HmacSha256::new_varkey(&key.to_bytes()).expect("HMAC can take key of any size");
        auth_token_raw.input(&data);
        let mut auth_token = [0u8; 32];
        auth_token.copy_from_slice(&auth_token_raw.result().code().to_vec());
        auth_token
    }
}

#[cfg(test)]
mod register_new_client {
    use super::*;

    // TODO: those tests require being called in async context. we need to research how to test this stuff...
//    #[test]
//    fn registers_new_auth_token_for_each_new_client() {
//        let req1 = RegisterRequest {
//            destination_address: [1u8; 32],
//        };
//        let registered_client_ledger = ClientLedger::new();
//        let store_dir = PathBuf::from("./foo/");
//        let key = Scalar::from_bytes_mod_order([1u8; 32]);
//        let client_processing_data = ClientProcessingData::new(store_dir, registered_client_ledger, key).add_arc_futures_mutex();
//
//
//        // need to do async....
//        client_processing_data.lock().await;
//        assert_eq!(0, registered_client_ledger.0.len());
//        ClientRequestProcessor::register_new_client(
//            req1,
//            client_processing_data.clone(),
//        );
//
//        assert_eq!(1, registered_client_ledger.0.len());
//
//        let req2 = RegisterRequest {
//            destination_address: [2u8; 32],
//        };
//        ClientRequestProcessor::register_new_client(
//            req2,
//            client_processing_data,
//        );
//        assert_eq!(2, registered_client_ledger.0.len());
//    }
//
//    #[test]
//    fn registers_given_token_only_once() {
//        let req1 = RegisterRequest {
//            destination_address: [1u8; 32],
//        };
//        let registered_client_ledger = ClientLedger::new();
//        let store_dir = PathBuf::from("./foo/");
//        let key = Scalar::from_bytes_mod_order([1u8; 32]);
//        let client_processing_data = ClientProcessingData::new(store_dir, registered_client_ledger, key).add_arc_futures_mutex();
//
//        ClientRequestProcessor::register_new_client(
//            req1,
//            client_processing_data.clone(),
//        );
//        let req2 = RegisterRequest {
//            destination_address: [1u8; 32],
//        };
//        ClientRequestProcessor::register_new_client(
//            req2,
//            client_processing_data.clone(),
//        );
//
//        client_processing_data.lock().await;
//
//        assert_eq!(1, registered_client_ledger.0.len())
//    }
}

#[cfg(test)]
mod create_storage_dir {
    use super::*;
    use sphinx::route::DestinationAddressBytes;

    #[test]
    fn it_creates_a_correct_storage_directory() {
        let client_address: DestinationAddressBytes = [1u8; 32];
        let store_dir = Path::new("./foo/");
        ClientRequestProcessor::create_storage_dir(client_address, store_dir);
    }
}

#[cfg(test)]
mod generating_new_auth_token {
    use super::*;

    #[test]
    fn for_the_same_input_generates_the_same_auth_token() {
        let data1 = vec![1u8; 55];
        let data2 = vec![1u8; 55];
        let key = Scalar::from_bytes_mod_order([1u8; 32]);
        let token1 = ClientRequestProcessor::generate_new_auth_token(data1, key);
        let token2 = ClientRequestProcessor::generate_new_auth_token(data2, key);
        assert_eq!(token1, token2);
    }

    #[test]
    fn for_different_inputs_generates_different_auth_tokens() {
        let data1 = vec![1u8; 55];
        let data2 = vec![2u8; 55];
        let key = Scalar::from_bytes_mod_order([1u8; 32]);
        let token1 = ClientRequestProcessor::generate_new_auth_token(data1, key);
        let token2 = ClientRequestProcessor::generate_new_auth_token(data2, key);
        assert_ne!(token1, token2);

        let data1 = vec![1u8; 50];
        let data2 = vec![2u8; 55];
        let key = Scalar::from_bytes_mod_order([1u8; 32]);
        let token1 = ClientRequestProcessor::generate_new_auth_token(data1, key);
        let token2 = ClientRequestProcessor::generate_new_auth_token(data2, key);
        assert_ne!(token1, token2);
    }
}
