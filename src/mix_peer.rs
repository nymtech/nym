use std::error::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::prelude::*;

#[derive(Debug)]
pub struct MixPeer {
    connection: SocketAddrV4,
}

impl MixPeer {
    // note that very soon `next_hop_address` will be changed to `next_hop_metadata`
    pub fn new(next_hop_address: [u8; 32]) -> MixPeer {
        let b = next_hop_address;
        let host = Ipv4Addr::new(b[0], b[1], b[2], b[3]);
        let port: u16 = u16::from_be_bytes([b[4], b[5]]);
        let socket_address = SocketAddrV4::new(host, port);
        MixPeer {
            connection: socket_address,
        }
    }

    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let next_hop_address = self.connection.clone();
        let mut stream = tokio::net::TcpStream::connect(next_hop_address).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }

    pub fn to_string(&self) -> String {
        self.connection.to_string()
    }
}
