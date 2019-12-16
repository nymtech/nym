use std::error::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::prelude::*;

pub struct MixPeer {
    connection: SocketAddrV4,
}

impl MixPeer {
    // note that very soon `next_hop_address` will be changed to `next_hop_metadata`
    pub fn new(next_hop_address: [u8; 32]) -> MixPeer {
        let address = String::from_utf8_lossy(&next_hop_address)
            .trim_end_matches(char::from(0))
            .to_string();
        MixPeer {
            connection: address,
        }
    }

    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let next_hop_address = self.connection.clone();
        let mut stream = tokio::net::TcpStream::connect(next_hop_address).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }
}
