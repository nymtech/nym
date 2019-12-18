use sfw_provider_requests::requests::{ProviderRequest, PullRequest, AuthToken, RegisterRequest};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse};
use std::net::{Shutdown, SocketAddr};
use std::net::SocketAddrV4;
use tokio::prelude::*;
use std::time::Duration;
use sphinx::route::DestinationAddressBytes;
use futures::io::Error;

#[derive(Debug)]
pub enum ProviderClientError {
    EmptyAuthTokenError,
    NetworkError,
}

impl From<io::Error> for ProviderClientError {
    fn from(_: Error) -> Self {
        use ProviderClientError::*;

        NetworkError
    }
}


pub struct ProviderClient {
    provider_network_address: SocketAddrV4,
    our_address: DestinationAddressBytes,
    auth_token: Option<AuthToken>,
}

impl ProviderClient {
    pub fn new(provider_network_address: SocketAddrV4, our_address: DestinationAddressBytes, auth_token: Option<AuthToken>) -> Self {
        // DH temporary: the provider's client port is not in the topology, but we can't change that
        // right now without messing up the existing Go mixnet. So I'm going to hardcode this
        // for the moment until the Go mixnet goes away.
        let provider_socket = SocketAddrV4::new(*provider_network_address.ip(), 9000);

        ProviderClient {
            provider_network_address: provider_socket,
            our_address,
            auth_token,
        }
    }

    pub async fn send_request(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ProviderClientError> {
        let mut socket = tokio::net::TcpStream::connect(self.provider_network_address).await?;
        println!("keep alive: {:?}", socket.keepalive());
        socket.set_keepalive(Some(Duration::from_secs(2))).unwrap();
        socket.write_all(&bytes[..]).await?;
        if let Err(e) = socket.shutdown(Shutdown::Write) {
            // TODO: make it a silent log once we have a proper logging library
//            eprintln!("failed to close write part of the socket; err = {:?}", e)
        }

        let mut response = Vec::new();
        socket.read_to_end(&mut response).await?;
        if let Err(e) = socket.shutdown(Shutdown::Read) {
            // TODO: make it a silent log once we have a proper logging library
//            eprintln!("failed to close read part of the socket; err = {:?}", e)
        }

        Ok(response)
    }

    pub async fn retrieve_messages(
        &self,
    ) -> Result<(), ProviderClientError> {
        if self.auth_token.is_none() {
            return Err(ProviderClientError::EmptyAuthTokenError)
        }

        let pull_request = PullRequest::new(self.our_address, self.auth_token.unwrap());
        let bytes = pull_request.to_bytes();

        let response = self.send_request(bytes).await?;
        println!("Received the following response: {:?}", response);
        
        let parsed_response = PullResponse::from_bytes(&response).unwrap();
        for message in parsed_response.messages {
            println!("Received: {:?}", String::from_utf8(message).unwrap())
        }
        Ok(())
    }

    pub async fn register(&mut self) -> Result<AuthToken, Box<dyn std::error::Error>> {
        let register_request = RegisterRequest::new(self.our_address);

        Ok([0;32])
    }
}
