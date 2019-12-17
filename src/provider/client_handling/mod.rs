use crate::provider::storage::{ClientStorage, StoreError};
use sfw_provider_requests::requests::{ProviderRequestError, ProviderRequests, PullRequest, RegisterRequest, ProviderRequest, AuthToken};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse, RegisterResponse};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use curve25519_dalek::digest::Digest;
use sha2::Sha256;
use std::io;
use std::collections::HashMap;
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug)]
pub enum ClientProcessingError {
    ClientDoesntExistError,
    StoreError,
    InvalidRequest,
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
    registered_clients_ledger: HashMap<Vec<u8>, Vec<u8>>,
}

impl ClientProcessingData {
    pub(crate) fn new(store_dir: PathBuf, registered_clients_ledger: HashMap<Vec<u8>, Vec<u8>>) -> Self {
        ClientProcessingData { store_dir,  registered_clients_ledger}
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
        let client_request = ProviderRequests::from_bytes(&data)?;
        println!("Received the following request: {:?}", client_request);
        match client_request {
            ProviderRequests::Register(req) => {
                Ok(ClientRequestProcessor::register_new_client(req, processing_data.read().unwrap().store_dir.as_path(),&mut processing_data.read().unwrap().registered_clients_ledger.clone())?.to_bytes())
            },
            ProviderRequests::PullMessages(req) => {
                Ok(ClientRequestProcessor::process_pull_messages_request(
                    req,
                    processing_data.read().unwrap().store_dir.as_path(),
                )?
                .to_bytes())
            }
        }
    }

    fn process_pull_messages_request(
        req: PullRequest,
        store_dir: &Path,
    ) -> Result<PullResponse, ClientProcessingError> {
        println!("Processing pull!");
        let retrieved_messages =
            ClientStorage::retrieve_client_files(req.destination_address, store_dir)?;
        Ok(PullResponse::new(retrieved_messages))
    }

    fn register_new_client(req:RegisterRequest, store_dir: &Path, registered_client_ledger: &mut HashMap<Vec<u8>, Vec<u8>>) -> Result<RegisterResponse, ClientProcessingError>{
        println!("Processing register new client request!");
        let auth_token = ClientRequestProcessor::generate_new_auth_token(req.destination_address.to_vec());
        registered_client_ledger.insert(auth_token.value.clone(), req.destination_address.to_vec());
        ClientRequestProcessor::create_storage_dir(req.destination_address, store_dir);
        Ok(RegisterResponse::new(auth_token.value))
    }

    fn create_storage_dir(client_address : sphinx::route::DestinationAddressBytes, store_dir: &Path) -> io::Result<()>{
        let client_dir_name = hex::encode(client_address);
        let full_store_dir = store_dir.join(client_dir_name);
        let full_store_path = full_store_dir.join(ClientStorage::generate_random_file_name());
        std::fs::create_dir_all(full_store_dir)?;
        Ok(())
    }



    fn generate_new_auth_token(data: Vec<u8>) -> AuthToken{
        // TODO: We can use hmac with providers secret key to have HMAC instead of SHA
        let mut sha256Hasher = Sha256::new();
        sha256Hasher.input(data);
        AuthToken{value: sha256Hasher.result().to_vec() }

    }

}

#[cfg(test)]
mod register_new_client {
    use super::*;

    #[test]
    fn registers_new_auth_token_for_each_new_client(){
        let req1 = RegisterRequest{destination_address: [1u8; 32]};
        let mut registered_client_ledger: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
        let store_dir = Path::new("./foo/bar.txt");
        assert_eq!(0, registered_client_ledger.len());
        ClientRequestProcessor::register_new_client(req1, &store_dir, &mut registered_client_ledger);
        assert_eq!(1, registered_client_ledger.len());

        let req2 = RegisterRequest{destination_address: [2u8; 32]};
        ClientRequestProcessor::register_new_client(req2, &store_dir, &mut registered_client_ledger);
        assert_eq!(2, registered_client_ledger.len());
    }

    #[test]
    fn registers_given_token_only_once() {
        let req1 = RegisterRequest{destination_address: [1u8; 32]};
        let mut registered_client_ledger: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
        let store_dir = Path::new("./foo/bar.txt");
        ClientRequestProcessor::register_new_client(req1, &store_dir, &mut registered_client_ledger);
        let req2 = RegisterRequest{destination_address: [1u8; 32]};
        ClientRequestProcessor::register_new_client(req2, &store_dir, &mut registered_client_ledger);
        assert_eq!(1, registered_client_ledger.len())
    }
}

#[cfg(test)]
mod generating_new_auth_token {
    use super::*;

    #[test]
    fn for_the_same_input_generates_the_same_auth_token(){
        let data1 = vec![1u8; 55];
        let data2 = vec![1u8; 55];
        let token1 = ClientRequestProcessor::generate_new_auth_token(data1);
        let token2 = ClientRequestProcessor::generate_new_auth_token(data2);
        assert_eq!(token1.value, token2.value);
    }

    #[test]
    fn for_different_inputs_generates_different_auth_tokens(){
        let data1 = vec![1u8; 55];
        let data2 = vec![2u8; 55];
        let token1 = ClientRequestProcessor::generate_new_auth_token(data1);
        let token2 = ClientRequestProcessor::generate_new_auth_token(data2);
        assert_ne!(token1.value, token2.value);

        let data1 = vec![1u8; 50];
        let data2 = vec![2u8; 55];
        let token1 = ClientRequestProcessor::generate_new_auth_token(data1);
        let token2 = ClientRequestProcessor::generate_new_auth_token(data2);
        assert_ne!(token1.value, token2.value);
    }


}