use sfw_provider_requests::requests::{ProviderRequest, PullRequest};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse};
use std::net::Shutdown;
use std::net::SocketAddrV4;
use tokio::prelude::*;
use tokio::time::Duration;

pub struct ProviderClient {}

impl ProviderClient {
    pub fn new() -> Self {
        ProviderClient {}
    }

    pub async fn retrieve_messages(
        &self,
        provider: &SocketAddrV4,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let address = [42; 32];
        let pull_request = PullRequest::new(address);
        let bytes = pull_request.to_bytes();

        let mut socket = tokio::net::TcpStream::connect(provider).await?;
        println!("keep alive: {:?}", socket.keepalive());
        socket.set_keepalive(Some(Duration::from_secs(2))).unwrap();
        socket.write_all(&bytes[..]).await?;
        if let Err(e) = socket.shutdown(Shutdown::Write) {
            eprintln!("failed to close write part of the socket; err = {:?}", e)
        }

        let mut response = Vec::new();
        socket.read_to_end(&mut response).await?;
        if let Err(e) = socket.shutdown(Shutdown::Read) {
            eprintln!("failed to close read part of the socket; err = {:?}", e)
        }

        println!("Received the following response: {:?}", response);
        let parsed_response = PullResponse::from_bytes(&response).unwrap();
        for message in parsed_response.messages {
            println!("Received: {:?}", String::from_utf8(message).unwrap())
        }
        Ok(())
    }
}
