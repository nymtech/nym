use addressing;
use addressing::AddressTypeError;
use sphinx::route::NodeAddressBytes;
use std::error::Error;
use std::net::SocketAddr;
use tokio::prelude::*;

#[derive(Debug)]
pub struct MixPeer {
    connection: SocketAddr,
}

#[derive(Debug)]
pub enum MixPeerError {
    InvalidAddressError,
}

impl From<addressing::AddressTypeError> for MixPeerError {
    fn from(_: AddressTypeError) -> Self {
        use MixPeerError::*;

        InvalidAddressError
    }
}

impl MixPeer {
    // note that very soon `next_hop_address` will be changed to `next_hop_metadata`
    pub fn new(next_hop_address: NodeAddressBytes) -> Result<MixPeer, MixPeerError> {
        let next_hop_socket_address =
            addressing::socket_address_from_encoded_bytes(next_hop_address.to_bytes())?;
        Ok(MixPeer {
            connection: next_hop_socket_address,
        })
    }

    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut stream = tokio::net::TcpStream::connect(self.connection).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }

    pub fn stringify(&self) -> String {
        self.connection.to_string()
    }
}
