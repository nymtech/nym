use sfw_provider_requests::requests::{ProviderRequest, PullRequest};
use sfw_provider_requests::responses::{ProviderResponse, PullResponse};
use std::net::{Shutdown, SocketAddr};
use std::net::SocketAddrV4;
use tokio::prelude::*;
use std::time::Duration;

pub struct ProviderClient {
    address: SocketAddrV4,
}

impl ProviderClient {
    pub fn new(address: SocketAddrV4) -> Self {
        ProviderClient {
            address
        }
    }

    pub async fn retrieve_messages(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let destination_address = [42u8; 32];
        let dummy_auth_token_to_make_it_compile = [0u8; 32];
        let pull_request = PullRequest::new(destination_address, dummy_auth_token_to_make_it_compile);
        let bytes = pull_request.to_bytes();

        // DH temporary: the provider's client port is not in the topology, but we can't change that
        // right now without messing up the existing Go mixnet. So I'm going to hardcode this
        // for the moment until the Go mixnet goes away.
        let provider_socket = SocketAddrV4::new(*self.address.ip(), 9000);
        println!("Provider: {:?}", provider_socket);

        let mut socket = tokio::net::TcpStream::connect(provider_socket).await?;
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
