use crate::provider::storage::{ClientStorage, StoreError};
use sfw_provider_requests::requests::{ProviderRequestError, ProviderRequests, PullRequest};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

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
    fn from(_: StoreError) -> Self {
        use ClientProcessingError::*;

        StoreError
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
            ProviderRequests::Register(req) => unimplemented!(),
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
}
