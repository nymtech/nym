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

use futures::io::Error;
use log::*;
use sfw_provider_requests::auth_token::AuthToken;
use sfw_provider_requests::requests::{
    async_io::TokioAsyncRequestWriter, ProviderRequest, PullRequest, RegisterRequest,
};
use sfw_provider_requests::responses::{
    async_io::TokioAsyncResponseReader, ProviderResponse, ProviderResponseError,
};
use sphinx::route::DestinationAddressBytes;
use std::net::SocketAddr;
use tokio::prelude::*;

#[derive(Debug)]
pub enum ProviderClientError {
    ClientAlreadyRegisteredError,
    EmptyAuthTokenError,
    NetworkError,

    InvalidRequestError,
    InvalidResponseError,
    InvalidResponseLengthError,
}

impl From<io::Error> for ProviderClientError {
    fn from(_: Error) -> Self {
        use ProviderClientError::*;

        NetworkError
    }
}

impl From<ProviderResponseError> for ProviderClientError {
    fn from(err: ProviderResponseError) -> Self {
        use ProviderClientError::*;
        match err {
            ProviderResponseError::MarshalError => InvalidRequestError,
            ProviderResponseError::UnmarshalError => InvalidResponseError,
            ProviderResponseError::UnmarshalErrorInvalidLength => InvalidResponseLengthError,
            ProviderResponseError::UnmarshalErrorInvalidKind => InvalidResponseLengthError,

            ProviderResponseError::TooLongResponseError => InvalidResponseError,
            ProviderResponseError::TooShortResponseError => InvalidResponseError,
            ProviderResponseError::IOError(_) => NetworkError,
            ProviderResponseError::RemoteConnectionClosed => NetworkError,
        }
    }
}

pub struct ProviderClient {
    provider_network_address: SocketAddr,
    our_address: DestinationAddressBytes,
    auth_token: Option<AuthToken>,
    connection: Option<tokio::net::TcpStream>,
    max_response_size: usize,
}

impl ProviderClient {
    pub fn new(
        provider_network_address: SocketAddr,
        our_address: DestinationAddressBytes,
        auth_token: Option<AuthToken>,
        max_response_size: usize,
    ) -> Self {
        ProviderClient {
            provider_network_address,
            our_address,
            auth_token,
            max_response_size,
            // establish connection when it's necessary (mainly to not break current code
            // as then 'new' would need to be called within async context)
            connection: None,
        }
    }

    async fn check_connection(&mut self) -> bool {
        if self.connection.is_some() {
            true
        } else {
            // TODO: possibly also introduce timeouts here?
            // However, at this point it's slightly less important as we are in full control
            // of providers.
            self.connection = tokio::net::TcpStream::connect(self.provider_network_address)
                .await
                .ok();
            self.connection.is_some()
        }
    }

    pub fn update_token(&mut self, auth_token: AuthToken) {
        self.auth_token = Some(auth_token)
    }

    pub async fn send_request(
        &mut self,
        request: ProviderRequest,
    ) -> Result<ProviderResponse, ProviderClientError> {
        if !self.check_connection().await {
            return Err(ProviderClientError::NetworkError);
        }

        let socket = self.connection.as_mut().unwrap();
        let (mut socket_reader, mut socket_writer) = socket.split();

        // TODO: benchmark and determine if below should be done:
        //        let mut socket_writer = tokio::io::BufWriter::new(socket_writer);
        //        let mut socket_reader = tokio::io::BufReader::new(socket_reader);

        let mut request_writer = TokioAsyncRequestWriter::new(&mut socket_writer);
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut socket_reader, self.max_response_size);

        if let Err(e) = request_writer.try_write_request(request).await {
            debug!("Failed to write the request - {:?}", e);
            return Err(e.into());
        }

        Ok(response_reader.try_read_response().await?)
    }

    pub async fn retrieve_messages(&mut self) -> Result<Vec<Vec<u8>>, ProviderClientError> {
        let auth_token = match self.auth_token.as_ref() {
            Some(token) => token.clone(),
            None => {
                return Err(ProviderClientError::EmptyAuthTokenError);
            }
        };

        let pull_request =
            ProviderRequest::Pull(PullRequest::new(self.our_address.clone(), auth_token));
        match self.send_request(pull_request).await? {
            ProviderResponse::Pull(res) => Ok(res.extract_messages()),
            ProviderResponse::Failure(res) => {
                error!(
                    "We failed to get our request processed - {:?}",
                    res.get_message()
                );
                Err(ProviderClientError::InvalidResponseError)
            }
            _ => {
                error!("Received response of unexpected type!");
                Err(ProviderClientError::InvalidResponseError)
            }
        }
    }

    pub async fn register(&mut self) -> Result<AuthToken, ProviderClientError> {
        if self.auth_token.is_some() {
            return Err(ProviderClientError::ClientAlreadyRegisteredError);
        }

        let register_request =
            ProviderRequest::Register(RegisterRequest::new(self.our_address.clone()));
        match self.send_request(register_request).await? {
            ProviderResponse::Register(res) => Ok(res.get_token()),
            ProviderResponse::Failure(res) => {
                error!(
                    "We failed to get our request processed - {:?}",
                    res.get_message()
                );
                Err(ProviderClientError::InvalidResponseError)
            }
            _ => {
                error!("Received response of unexpected type!");
                Err(ProviderClientError::InvalidResponseError)
            }
        }
    }

    pub fn is_registered(&self) -> bool {
        self.auth_token.is_some()
    }
}
