use tokio::net::TcpStream;
use tokio::prelude::*;
use std::error::Error;


pub struct MixPeer {
    connection: &'static str,
}

impl MixPeer {
    pub fn new() -> MixPeer {
        let next_hop_address = "127.0.0.1:8081";
        let node = MixPeer {
            connection: next_hop_address,
        };
        node
    }

    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), Box<dyn Error>>{
        let mut stream = TcpStream::connect(self.connection).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }
}