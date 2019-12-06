use std::error::Error;

use tokio::net::TcpStream;
use tokio::prelude::*;

pub struct MixPeer<'a> {
    connection: &'a str,
}

impl<'a> MixPeer<'a> {
    // for now completely ignore data we're sending.
    // also note that very soon next_hop_address will be changed to next_hop_metadata
//    pub fn new(nex_hop_address: Vec<u8>) -> MixPeer<'a> {
    pub fn new(next_hop_address: [u8; 32]) -> MixPeer<'a> {
        let next_hop_address_fixture: &'a str = "127.0.0.1:8081";
        MixPeer {
            connection: next_hop_address_fixture,
        }
    }

    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut stream = TcpStream::connect(self.connection).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }
}