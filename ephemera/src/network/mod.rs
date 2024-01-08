use std::fmt::Display;
use std::net::IpAddr;
use std::str::FromStr;

use ::libp2p::{multiaddr::Protocol, Multiaddr};
use libp2p_identity::PeerId as Libp2pPeerId;
use log::info;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::crypto::PublicKey;

pub(crate) mod libp2p;
pub(crate) mod members;

pub(crate) type PeerIdType = Libp2pPeerId;

#[derive(Debug, Error)]
pub enum PeerIdError {
    #[error("Invalid peer ID: {0}")]
    InvalidPeerId(String),
}

/// Unique identifier of a peer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub(crate) PeerIdType);

impl PeerId {
    #[must_use]
    pub fn random() -> Self {
        Self(PeerIdType::random())
    }

    /// Returns the internal representation of the peer ID.
    pub(crate) fn inner(&self) -> &PeerIdType {
        &self.0
    }

    /// Returns a raw representation of the peer ID.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    /// Returns a peer ID from a raw representation.
    ///
    /// # Returns
    /// A `PeerId` if the bytes are valid.
    ///
    /// # Errors
    /// An error if input has wrong format. This function is reverse to [`PeerId::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PeerIdError> {
        Ok(Self(PeerIdType::from_bytes(bytes).map_err(|e| {
            PeerIdError::InvalidPeerId(format!("Invalid peer ID: {e}"))
        })?))
    }

    /// Builds a `PeerId` from a public key.
    #[must_use]
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        Self(PeerIdType::from_public_key(public_key.inner()))
    }
}

impl From<PeerId> for libp2p_identity::PeerId {
    fn from(peer_id: PeerId) -> Self {
        peer_id.0
    }
}

impl From<libp2p_identity::PeerId> for PeerId {
    fn from(peer_id: libp2p_identity::PeerId) -> Self {
        Self(peer_id)
    }
}

impl Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait ToPeerId {
    fn peer_id(&self) -> PeerId;
}

/// A peer of the network.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Peer {
    /// The peer's ID. It identifies the peer uniquely and is derived from its public key.
    ///
    /// # Deriving PeerId from PublicKey example
    ///
    /// ```
    /// use ephemera::crypto::{EphemeraKeypair, Keypair, PublicKey};
    /// use ephemera::peer::{PeerId, ToPeerId};
    ///
    /// let public_key = Keypair::generate(None).public_key();
    ///
    /// let peer_id = PeerId::from_public_key(&public_key);
    ///
    /// assert_eq!(peer_id, public_key.peer_id());
    ///
    /// ```
    #[allow(clippy::struct_field_names)]
    // this should get resolved properly at some point, but not now...
    pub peer_id: PeerId,
    /// The peer's public key. It matches PeerId.
    pub public_key: PublicKey,
    /// The peer's address.
    pub address: Address,
    /// The cosmos address of the peer, used in interacting with the chain.
    pub cosmos_address: String,
}

#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Failed to parse address: {0}")]
    ParsingError(String),
}

/// Ephemera node address.
///
/// Supported formats:
/// 1. `<IP>:<PORT>`
/// 2. `/ip4/<IP>/tcp/<PORT>` - this is format used by libp2p multiaddr.
/// 3. `/dns4/<NAME>/tcp/<PORT>` - this is format used by libp2p multiaddr.
/// See [libp2p/multiaddress](https://github.com/libp2p/specs/blob/master/addressing/README.md) for more details.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(pub Multiaddr);

impl Address {
    pub fn inner(&self) -> &Multiaddr {
        &self.0
    }
}

impl From<Multiaddr> for Address {
    fn from(multiaddr: Multiaddr) -> Self {
        Self(multiaddr)
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let address: Option<Multiaddr> = match Multiaddr::from_str(s) {
            Ok(multiaddr) => Some(multiaddr),
            Err(err) => {
                info!("Failed to parse multiaddr: {}", err);
                None
            }
        };

        let multi_address = address.or_else(|| match std::net::SocketAddr::from_str(s) {
            Ok(sa) => {
                let mut multiaddr = Multiaddr::empty();
                match sa {
                    std::net::SocketAddr::V4(v4) => {
                        multiaddr.push(Protocol::Ip4(*v4.ip()));
                        multiaddr.push(Protocol::Tcp(v4.port()));
                    }
                    std::net::SocketAddr::V6(v6) => {
                        multiaddr.push(Protocol::Ip6(*v6.ip()));
                        multiaddr.push(Protocol::Tcp(v6.port()));
                    }
                }

                Some(multiaddr)
            }
            Err(err) => {
                info!("Failed to parse socket addr: {err}");
                None
            }
        });

        match multi_address {
            Some(multi_address) => Ok(Self(multi_address)),
            None => Err(AddressError::ParsingError(s.to_string())),
        }
    }
}

impl TryFrom<Address> for (IpAddr, u16) {
    type Error = std::io::Error;

    fn try_from(addr: Address) -> Result<Self, Self::Error> {
        let mut multiaddr = addr.0;
        if let Some(Protocol::Tcp(port)) = multiaddr.pop() {
            if let Some(Protocol::Ip4(ip)) = multiaddr.pop() {
                return Ok((IpAddr::V4(ip), port));
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "invalid address",
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_multiaddr() {
        "/ip4/127.0.0.1/tcp/1234".parse::<Address>().unwrap();
    }

    #[test]
    fn test_parse_ip_port() {
        "127.0.0.1:1234".parse::<Address>().unwrap();
    }

    #[test]
    fn test_fail_parse_multiaddr_without_port() {
        let result = "/ip4/127.0.0.1/tcp/".parse::<Address>();
        assert!(matches!(result, Err(AddressError::ParsingError(_))));
    }

    #[test]
    fn test_fail_parse_multiaddr_without_ip() {
        let result = "/ip4//tcp/1234".parse::<Address>();
        assert!(matches!(result, Err(AddressError::ParsingError(_))));
    }

    #[test]
    fn test_fail_parse_ip_port_without_port() {
        let result = "127.0.0.1".parse::<Address>();
        assert!(matches!(result, Err(AddressError::ParsingError(_))));
    }

    #[test]
    fn test_fail_parse_ip_port_without_ip() {
        let result = "1234".parse::<Address>();
        assert!(matches!(result, Err(AddressError::ParsingError(_))));
    }
}
