use futures::io::Error;
use log::*;
use sfw_provider_requests::requests::{ProviderRequest, PullRequest, RegisterRequest};
use sfw_provider_requests::responses::{
    ProviderResponse, ProviderResponseError, PullResponse, RegisterResponse,
};
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use std::net::{Shutdown, SocketAddr};
use std::time::Duration;
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
        }
    }
}

pub struct ProviderClient {
    provider_network_address: SocketAddr,
    our_address: DestinationAddressBytes,
    auth_token: Option<AuthToken>,
}

impl ProviderClient {
    pub fn new(
        provider_network_address: SocketAddr,
        our_address: DestinationAddressBytes,
        auth_token: Option<AuthToken>,
    ) -> Self {
        ProviderClient {
            provider_network_address,
            our_address,
            auth_token,
        }
    }

    pub fn update_token(&mut self, auth_token: AuthToken) {
        self.auth_token = Some(auth_token)
    }

    pub async fn send_request(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ProviderClientError> {
        let mut socket = tokio::net::TcpStream::connect(self.provider_network_address).await?;

        socket.set_keepalive(Some(Duration::from_secs(2)))?;
        socket.write_all(&bytes[..]).await?;
        if let Err(e) = socket.shutdown(Shutdown::Write) {
            warn!("failed to close write part of the socket; err = {:?}", e)
        }

        let mut response = Vec::new();
        socket.read_to_end(&mut response).await?;
        if let Err(e) = socket.shutdown(Shutdown::Read) {
            debug!("failed to close read part of the socket; err = {:?}. It was probably already closed by the provider", e)
        }

        Ok(response)
    }

    pub async fn retrieve_messages(&self) -> Result<Vec<Vec<u8>>, ProviderClientError> {
        let auth_token = match self.auth_token.as_ref() {
            Some(token) => token.clone(),
            None => {
                return Err(ProviderClientError::EmptyAuthTokenError);
            }
        };

        let pull_request = PullRequest::new(self.our_address.clone(), auth_token);
        let bytes = pull_request.to_bytes();

        let response = self.send_request(bytes).await?;

        let parsed_response = PullResponse::from_bytes(&response)?;
        Ok(parsed_response.messages)
    }

    pub async fn register(&self) -> Result<AuthToken, ProviderClientError> {
        if self.auth_token.is_some() {
            return Err(ProviderClientError::ClientAlreadyRegisteredError);
        }

        let register_request = RegisterRequest::new(self.our_address.clone());
        let bytes = register_request.to_bytes();

        let response = self.send_request(bytes).await?;
        let parsed_response = RegisterResponse::from_bytes(&response)?;

        Ok(parsed_response.auth_token)
    }

    pub fn is_registered(&self) -> bool {
        self.auth_token.is_some()
    }
}
