// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::provider::client_handling::ledger::ClientLedger;
use crate::provider::storage::{ClientFile, ClientStorage};
use crypto::encryption;
use hmac::{Hmac, Mac};
use log::*;
use nymsphinx::DestinationAddressBytes;
use sfw_provider_requests::auth_token::AuthToken;
use sfw_provider_requests::requests::*;
use sha2::Sha256;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub enum ClientProcessingResult {
    PullResponse(Vec<ClientFile>),
    RegisterResponse(AuthToken),
}

#[derive(Debug)]
pub enum ClientProcessingError {
    InvalidRequest,
    InvalidToken,
    IOError(String),
}

impl From<ProviderRequestError> for ClientProcessingError {
    fn from(_: ProviderRequestError) -> Self {
        use ClientProcessingError::*;

        InvalidRequest
    }
}

impl From<io::Error> for ClientProcessingError {
    fn from(e: io::Error) -> Self {
        use ClientProcessingError::*;

        IOError(e.to_string())
    }
}

// PacketProcessor contains all data required to correctly process client requests
#[derive(Clone)]
pub struct RequestProcessor {
    secret_key: Arc<encryption::PrivateKey>,
    client_storage: ClientStorage,
    client_ledger: ClientLedger,
    max_request_size: usize,
}

impl RequestProcessor {
    pub(crate) fn new(
        secret_key: encryption::PrivateKey,
        client_storage: ClientStorage,
        client_ledger: ClientLedger,
        max_request_size: usize,
    ) -> Self {
        RequestProcessor {
            secret_key: Arc::new(secret_key),
            client_storage,
            client_ledger,
            max_request_size,
        }
    }

    pub(crate) fn max_request_size(&self) -> usize {
        self.max_request_size
    }

    pub(crate) async fn process_client_request(
        &mut self,
        client_request: ProviderRequest,
    ) -> Result<ClientProcessingResult, ClientProcessingError> {
        match client_request {
            ProviderRequest::Register(req) => self.process_register_request(req).await,
            ProviderRequest::Pull(req) => self.process_pull_request(req).await,
        }
    }

    pub(crate) async fn process_register_request(
        &mut self,
        req: RegisterRequest,
    ) -> Result<ClientProcessingResult, ClientProcessingError> {
        debug!(
            "Processing register new client request: {:?}",
            req.destination_address.to_base58_string()
        );

        let auth_token = self.generate_new_auth_token(req.destination_address.clone());
        if self
            .client_ledger
            .insert_token(auth_token.clone(), req.destination_address.clone())
            .unwrap()
            .is_some()
        {
            info!(
                "Client {:?} was already registered before!",
                req.destination_address.to_base58_string()
            )
        } else if let Err(e) = self
            .client_storage
            .create_storage_dir(req.destination_address.clone())
            .await
        {
            error!("We failed to create inbox directory for the client -{:?}\nReverting issued token...", e);
            // we must revert our changes if this operation failed
            self.client_ledger
                .remove_token(&req.destination_address)
                .unwrap();
        }

        Ok(ClientProcessingResult::RegisterResponse(auth_token))
    }

    fn generate_new_auth_token(&self, client_address: DestinationAddressBytes) -> AuthToken {
        type HmacSha256 = Hmac<Sha256>;

        // note that `new_varkey` doesn't even have an execution branch returning an error
        // (true as of hmac 0.7.1)
        let mut auth_token_raw = HmacSha256::new_varkey(&self.secret_key.to_bytes()).unwrap();
        auth_token_raw.input(client_address.as_bytes());
        let mut auth_token = [0u8; 32];
        auth_token.copy_from_slice(auth_token_raw.result().code().as_slice());
        AuthToken::from_bytes(auth_token)
    }

    pub(crate) async fn process_pull_request(
        &self,
        req: PullRequest,
    ) -> Result<ClientProcessingResult, ClientProcessingError> {
        debug!(
            "Processing a pull request from {:?}",
            req.destination_address.to_base58_string()
        );
        if self
            .client_ledger
            .verify_token(&req.auth_token, &req.destination_address)
            .unwrap()
        {
            let retrieved_messages = self
                .client_storage
                .retrieve_client_files(req.destination_address)
                .await?;
            Ok(ClientProcessingResult::PullResponse(retrieved_messages))
        } else {
            Err(ClientProcessingError::InvalidToken)
        }
    }

    pub(crate) async fn delete_sent_messages(&self, file_paths: Vec<PathBuf>) -> io::Result<()> {
        self.client_storage.delete_files(file_paths).await
    }
}

#[cfg(test)]
mod generating_new_auth_token {
    use super::*;

    #[test]
    fn for_the_same_input_generates_the_same_auth_token() {
        let client_address1 = DestinationAddressBytes::from_bytes([1; 32]);
        let client_address2 = DestinationAddressBytes::from_bytes([1; 32]);
        let key = encryption::PrivateKey::from_bytes(&[2u8; 32]);

        let request_processor = RequestProcessor {
            secret_key: Arc::new(key),
            client_storage: ClientStorage::new(3, 16, Default::default()),
            client_ledger: ClientLedger::create_temporary(),
            max_request_size: 42,
        };

        let token1 = request_processor.generate_new_auth_token(client_address1);
        let token2 = request_processor.generate_new_auth_token(client_address2);
        assert_eq!(token1, token2);
    }

    #[test]
    fn for_different_inputs_generates_different_auth_tokens() {
        let client_address1 = DestinationAddressBytes::from_bytes([1; 32]);
        let client_address2 = DestinationAddressBytes::from_bytes([2; 32]);
        let key1 = encryption::PrivateKey::from_bytes(&[3u8; 32]);
        let key2 = encryption::PrivateKey::from_bytes(&[4u8; 32]);

        let request_processor1 = RequestProcessor {
            secret_key: Arc::new(key1),
            client_storage: ClientStorage::new(3, 16, Default::default()),
            client_ledger: ClientLedger::create_temporary(),
            max_request_size: 42,
        };

        let request_processor2 = RequestProcessor {
            secret_key: Arc::new(key2),
            client_storage: ClientStorage::new(3, 16, Default::default()),
            client_ledger: ClientLedger::create_temporary(),
            max_request_size: 42,
        };

        let token1 = request_processor1.generate_new_auth_token(client_address1.clone());
        let token2 = request_processor1.generate_new_auth_token(client_address2.clone());

        let token3 = request_processor2.generate_new_auth_token(client_address1);
        let token4 = request_processor2.generate_new_auth_token(client_address2);

        assert_ne!(token1, token2);
        assert_ne!(token1, token3);
        assert_ne!(token1, token4);
        assert_ne!(token2, token3);
        assert_ne!(token2, token4);
        assert_ne!(token3, token4);
    }
}
