use sphinx::route::Node as MixNode;
use sphinx::SphinxPacket;
use tokio::prelude::*;
use sfw_provider_requests::*;
use std::net::Shutdown;

pub struct ProviderClient {}


impl ProviderClient {
    pub fn new() -> Self {
        ProviderClient {}
    }

    pub async fn send(
        &self,
//        provider: &MixNode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let address = [0; 32];
        let pull_request = PullRequest::new(address);
        let bytes = pull_request.to_bytes();

        let mut socket = tokio::net::TcpStream::connect("127.0.0.1:9000").await?;
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

        Ok(())
    }
}
