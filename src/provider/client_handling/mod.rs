use crate::provider::storage::{ClientStorage, StoreError};
use curve25519_dalek::digest::Digest;
use curve25519_dalek::scalar::Scalar;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sfw_provider_requests::requests::{
    AuthToken, ProviderRequest, ProviderRequestError, ProviderRequests, PullRequest,
    RegisterRequest,
};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse, RegisterResponse};
use sha2::Sha256;
use sphinx::route::DestinationAddressBytes;
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub enum ClientProcessingError {
    ClientDoesntExistError,
    StoreError,
    InvalidRequest,
    WrongToken,
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

#[derive(Debug, Clone)]
pub(crate) struct ClientProcessingData {
    store_dir: PathBuf,
    registered_clients_ledger: HashMap<AuthToken, DestinationAddressBytes>,
    secret_key: Scalar,
}

impl ClientProcessingData {
    pub(crate) fn new(
        store_dir: PathBuf,
        registered_clients_ledger: HashMap<AuthToken, DestinationAddressBytes>,
        secret_key: Scalar,
    ) -> Self {
        ClientProcessingData {
            store_dir,
            registered_clients_ledger,
            secret_key,
        }
    }

    pub(crate) fn add_arc_rwlock(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }
}

pub(crate) struct ClientRequestProcessor(());

impl ClientRequestProcessor {
    pub(crate) fn process_client_request(
        data: &[u8],
        processing_data: &RwLock<ClientProcessingData>,
    ) -> Result<Vec<u8>, ClientProcessingError> {
        let read_processing_data = processing_data.read().unwrap();
        let client_request = ProviderRequests::from_bytes(&data)?;
        println!("Received the following request: {:?}", client_request);
        match client_request {
            ProviderRequests::Register(req) => Ok(ClientRequestProcessor::register_new_client(
                req,
                read_processing_data.store_dir.as_path(),
                // TODO: this WILL NOT work because ledger is cloned
                &mut processing_data
                    .read()
                    .unwrap()
                    .registered_clients_ledger
                    .clone(),
                read_processing_data.secret_key,
            )?
            .to_bytes()),
            ProviderRequests::PullMessages(req) => {
                Ok(ClientRequestProcessor::process_pull_messages_request(
                    req,
                    processing_data.read().unwrap().store_dir.as_path(),
                    &mut processing_data
                        .read()
                        .unwrap()
                        .registered_clients_ledger
                        .clone(),
                )?
                .to_bytes())
            }
        }
    }

    fn process_pull_messages_request(
        req: PullRequest,
        store_dir: &Path,
        registered_client_ledger: &mut HashMap<AuthToken, DestinationAddressBytes>,
    ) -> Result<PullResponse, ClientProcessingError> {
        println!("Processing pull!");
        if registered_client_ledger.contains_key(&req.auth_token) {
            let retrieved_messages =
                ClientStorage::retrieve_client_files(req.destination_address, store_dir)?;
            Ok(PullResponse::new(retrieved_messages))
        } else {
            Err(ClientProcessingError::WrongToken)
        }
    }

    fn register_new_client(
        req: RegisterRequest,
        store_dir: &Path,
        registered_client_ledger: &mut HashMap<AuthToken, DestinationAddressBytes>,
        provider_secret_key: Scalar,
    ) -> Result<RegisterResponse, ClientProcessingError> {
        println!("Processing register new client request!");
        let auth_token = ClientRequestProcessor::generate_new_auth_token(
            req.destination_address.to_vec(),
            provider_secret_key,
        );
        if !registered_client_ledger.contains_key(&auth_token) {
            registered_client_ledger.insert(auth_token, req.destination_address);
            ClientRequestProcessor::create_storage_dir(req.destination_address, store_dir);
        }
        Ok(RegisterResponse::new(auth_token.to_vec()))
    }

    fn create_storage_dir(
        client_address: sphinx::route::DestinationAddressBytes,
        store_dir: &Path,
    ) -> io::Result<()> {
        let client_dir_name = hex::encode(client_address);
        let full_store_dir = store_dir.join(client_dir_name);
        let full_store_path = full_store_dir.join(ClientStorage::generate_random_file_name());
        std::fs::create_dir_all(full_store_dir)?;
        Ok(())
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

    #[test]
    fn registers_new_auth_token_for_each_new_client() {
        let req1 = RegisterRequest {
            destination_address: [1u8; 32],
        };
        let mut registered_client_ledger: HashMap<AuthToken, DestinationAddressBytes> =
            HashMap::new();
        let store_dir = Path::new("./foo/");
        let key = Scalar::from_bytes_mod_order([1u8; 32]);
        assert_eq!(0, registered_client_ledger.len());
        ClientRequestProcessor::register_new_client(
            req1,
            &store_dir,
            &mut registered_client_ledger,
            key,
        );
        assert_eq!(1, registered_client_ledger.len());

        let req2 = RegisterRequest {
            destination_address: [2u8; 32],
        };
        ClientRequestProcessor::register_new_client(
            req2,
            &store_dir,
            &mut registered_client_ledger,
            key,
        );
        assert_eq!(2, registered_client_ledger.len());
    }

    #[test]
    fn registers_given_token_only_once() {
        let req1 = RegisterRequest {
            destination_address: [1u8; 32],
        };
        let mut registered_client_ledger: HashMap<AuthToken, DestinationAddressBytes> =
            HashMap::new();
        let store_dir = Path::new("./foo/");
        let key = Scalar::from_bytes_mod_order([1u8; 32]);
        ClientRequestProcessor::register_new_client(
            req1,
            &store_dir,
            &mut registered_client_ledger,
            key,
        );
        let req2 = RegisterRequest {
            destination_address: [1u8; 32],
        };
        ClientRequestProcessor::register_new_client(
            req2,
            &store_dir,
            &mut registered_client_ledger,
            key,
        );
        assert_eq!(1, registered_client_ledger.len())
    }
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
