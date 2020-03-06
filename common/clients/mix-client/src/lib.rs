use log::*;
use sphinx::SphinxPacket;
use std::net::SocketAddr;
use tokio::prelude::*;

pub mod packet;
pub mod poisson;

pub struct MixClient {}

impl MixClient {
    #[allow(clippy::new_without_default)]
    pub fn new() -> MixClient {
        MixClient {}
    }

    // Sends a Sphinx packet to a mixnode.
    pub async fn send(
        &self,
        packet: SphinxPacket,
        mix_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = packet.to_bytes();
        debug!("Sending to the following address: {:?}", mix_addr);

        let mut stream = tokio::net::TcpStream::connect(mix_addr).await?;
        stream.write_all(&bytes[..]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod sending_a_sphinx_packet {
    // use super::*;
    // use sphinx::SphinxPacket;

    #[test]
    fn works() {
        // arrange
        //        let directory = Client::new();
        //        let message = "Hello, Sphinx!".as_bytes().to_vec();
        //        let mixes = directory.get_mixes();
        //        let destination = directory.get_destination();
        //        let delays = sphinx::header::delays::generate(2);
        //        let packet = SphinxPacket::new(message, &mixes, &destination, &delays).unwrap();
        //        let mix_client = MixClient::new();
        //        let first_hop = mixes.first().unwrap();
        //
        //        // act
        //        mix_client.send(packet, first_hop);

        // assert
        // wtf are we supposed to assert here?
    }
}
