use sphinx::route::Node as MixNode;
use sphinx::SphinxPacket;
use tokio::prelude::*;
use sfw_provider_requests::*;

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

        let mut stream = tokio::net::TcpStream::connect("127.0.0.1:9000").await?;
        stream.write_all(&bytes[..]).await?;
        Ok(())
    }
}
