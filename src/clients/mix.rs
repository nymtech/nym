use crate::clients::directory::DirectoryClient;
use sphinx::SphinxPacket;
use sphinx::route::Node as MixNode;
use tokio::net::TcpStream;
use tokio::prelude::*;

pub struct MixClient {}

impl MixClient {
    pub fn new() -> MixClient {
        MixClient {}
    }

    pub async fn send(&self, packet: SphinxPacket, mix: &MixNode) -> Result<(), Box<dyn std::error::Error>>{
        let bytes = packet.to_bytes();

        // now we shoot it into space!
        let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
        stream.write_all(&bytes[..]).await?;
        Ok(())
    }
}


#[cfg(test)]
mod sending_a_sphinx_packet {
    use super::*;
    use sphinx::SphinxPacket;

    #[test]
    fn works() {
        // arrange
        let directory = DirectoryClient::new();
        let message = "Hello, Sphinx!".as_bytes().to_vec();
        let mixes = directory.get_mixes();
        let destination = directory.get_destination();
        let packet = SphinxPacket::new(message, &mixes, &destination);
        let mix_client = MixClient::new();
        let first_hop = mixes.first().unwrap();

        // act
        mix_client.send(packet, first_hop);

        // assert
        // wtf are we supposed to assert here?
    }
}