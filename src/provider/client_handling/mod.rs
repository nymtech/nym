use crate::provider::storage::{ClientStorage, StoreError};
use sfw_provider_requests::requests::{ProviderRequestError, ProviderRequests, PullRequest, RegisterRequest, ProviderRequest};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse, RegisterResponse};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use curve25519_dalek::digest::Digest;
use sha2::Sha256;
use std::io;
use std::collections::HashMap;

#[derive(Debug)]
pub enum ClientProcessingError {
    ClientDoesntExistError,
    StoreError,
    InvalidRequest,
}

struct AuthToken {
    value : Vec<u8>
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
}

impl ClientProcessingData {
    pub(crate) fn new(store_dir: PathBuf) -> Self {
        ClientProcessingData { store_dir }
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
        println!("received the following request: {:?}", client_request);
        match client_request {
            ProviderRequests::Register(req) => {
                Ok(ClientRequestProcessor::register_new_client(req)?.to_bytes())
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
        println!("processing pull!");
        let retrieved_messages =
            ClientStorage::retrieve_client_files(req.destination_address, store_dir)?;
        Ok(PullResponse::new(retrieved_messages))
    }

    fn register_new_client(req:RegisterRequest) -> Result<RegisterResponse, ClientProcessingError>{
        let auth_token = ClientRequestProcessor::generate_new_auth_token(req.destination_address.to_vec());
        //somehow register token
        Ok(RegisterResponse::new(auth_token.value))

    }

    fn generate_new_auth_token(data: Vec<u8>) -> AuthToken{
        // TODO: We can use hmac with providers secret key to have HMAC instead of SHA
        let mut sha256Hasher = Sha256::new();
        sha256Hasher.input(data);
        AuthToken{value: sha256Hasher.result().to_vec() }

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