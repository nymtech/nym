// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Encoding and decoding node routing information.
//!
//! This module is responsible for encoding and decoding node routing information, so that
//! they could be later put into an appropriate field in a sphinx header.
//! A routing address is either a `SocketAddr` (mix node / gateway socket) or a `ClientAddress`
//! (a 20-byte fingerprint or a client's identity key.

use crate::clients::ClientAddress;
use nym_crypto::asymmetric::ed25519;
use nym_sphinx_types::{NODE_ADDRESS_LENGTH, NodeAddressBytes};

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use thiserror::Error;

// Not entirely sure whether this is the correct place for those, but let's see how it's going
// to work out
pub type NodeIdentity = ed25519::PublicKey;
pub const NODE_IDENTITY_SIZE: usize = ed25519::PUBLIC_KEY_LENGTH;

/// MAX_UNPADDED_LEN represents maximum length an unpadded address could have.
/// Reserved for ACK-SURB hop overhead calculations, which only ever target mix nodes,
/// so the cap is the IPv6 socket variant (1 + 2 + 16 = 19 bytes).
pub const MAX_NODE_ADDRESS_UNPADDED_LEN: usize = 19;

#[derive(Debug, Error)]
pub enum NymNodeRoutingAddressError {
    #[error("Attempted to deserialize NymNodeRoutingAddress without providing any bytes")]
    NoBytesProvided,

    #[error(
        "Provided insufficient amount of few bytes to deserialize a valid NymNodeRoutingAddress for type {address_type}. Received {received} and required {required}"
    )]
    TooFewBytesProvided {
        address_type: AddressType,
        received: usize,
        required: usize,
    },

    #[error("{received:#x} is not a valid NymNodeRoutingAddress address type")]
    InvalidAddressType { received: u8 },

    #[error(
        "Could not serialize NymNodeRoutingAddress into NodeAddressBytes as that requires using at least {required} bytes and only {NODE_ADDRESS_LENGTH} are available"
    )]
    TooSmallBytesRepresentation { required: usize },
}

/// On-wire variant tag of a [`NymNodeRoutingAddress`]. Always the first byte
/// of the encoded routing field.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum AddressType {
    Ipv4 = 4,
    Ipv6 = 6,
    Client = 12,
}

impl AddressType {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Number of bytes needed to represent the address type.
    pub fn bytes_len(&self) -> usize {
        match self {
            AddressType::Ipv4 => 1 + 4 + 2,  // marker, address, port
            AddressType::Ipv6 => 1 + 16 + 2, // marker, address, port
            AddressType::Client => 1 + ClientAddress::LEN, // marker, fingerprint
        }
    }
}

impl TryFrom<u8> for AddressType {
    type Error = NymNodeRoutingAddressError;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            x if x == AddressType::Ipv4 as u8 => Ok(AddressType::Ipv4),
            x if x == AddressType::Ipv6 as u8 => Ok(AddressType::Ipv6),
            x if x == AddressType::Client as u8 => Ok(AddressType::Client),
            v => Err(NymNodeRoutingAddressError::InvalidAddressType { received: v }),
        }
    }
}

/// Routing information that can appear in a sphinx hop's address field.
///
/// `Node` carries an inter-node socket address (mix node or gateway egress).
/// `Client` carries a 20-byte fingerprint of the destination client's identity
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum NymNodeRoutingAddress {
    Node(SocketAddr),
    Client(ClientAddress),
}

impl std::fmt::Display for NymNodeRoutingAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NymNodeRoutingAddress::Node(addr) => addr.fmt(f),
            NymNodeRoutingAddress::Client(ca) => ca.fmt(f),
        }
    }
}

impl NymNodeRoutingAddress {
    /// Minimum number of bytes that need to be available to represent self.
    /// The value has no upper bound as when converted into bytes, it's always
    /// padded with zeroes to be exactly NODE_ADDRESS_LENGTH long.
    pub fn bytes_min_len(&self) -> usize {
        self.addr_type().bytes_len()
    }

    /// Converts self into a vector of bytes.
    /// Note, this represents a generic bytes vector, not necessarily a NodeAddressBytes
    /// and hence is not zero-padded.
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            NymNodeRoutingAddress::Node(socket) => {
                let port_bytes = socket.port().to_be_bytes();
                let ip_octets_vec = match socket.ip() {
                    IpAddr::V4(ip) => ip.octets().to_vec(),
                    IpAddr::V6(ip) => ip.octets().to_vec(),
                };
                std::iter::once(self.addr_type().as_u8())
                    .chain(port_bytes.iter().cloned())
                    .chain(ip_octets_vec.iter().cloned())
                    .collect()
            }
            NymNodeRoutingAddress::Client(address) => std::iter::once(self.addr_type().as_u8())
                .chain(address.to_bytes())
                .collect(),
        }
    }

    /// Converts self into a vector of bytes optionally padded with zeroes to the `expected_len`.
    /// Note this does not necessarily represent a NodeAddressBytes, unless
    /// `expected_len` == NODE_ADDRESS_LENGTH
    pub fn as_zero_padded_bytes(&self, expected_len: usize) -> Vec<u8> {
        let self_bytes = self.as_bytes();
        if self_bytes.len() >= expected_len {
            // can't add padding
            self_bytes
        } else {
            self_bytes
                .into_iter()
                .chain(std::iter::repeat(0))
                .take(expected_len)
                .collect()
        }
    }

    /// Tries to recover `Self` from a bytes slice.
    /// Does not care if it's zero-padded or not.
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, NymNodeRoutingAddressError> {
        if b.is_empty() {
            return Err(NymNodeRoutingAddressError::NoBytesProvided);
        }

        let address_type = AddressType::try_from(b[0])?;
        if b.len() < address_type.bytes_len() {
            return Err(NymNodeRoutingAddressError::TooFewBytesProvided {
                address_type,
                received: b.len(),
                required: address_type.bytes_len(),
            });
        }

        match address_type {
            AddressType::Ipv4 => {
                let port = u16::from_be_bytes([b[1], b[2]]);
                let ip = IpAddr::V4(Ipv4Addr::new(b[3], b[4], b[5], b[6]));
                Ok(NymNodeRoutingAddress::Node(SocketAddr::new(ip, port)))
            }
            AddressType::Ipv6 => {
                let port = u16::from_be_bytes([b[1], b[2]]);
                let mut address_octets = [0u8; 16];
                address_octets.copy_from_slice(&b[3..19]);
                let ip = IpAddr::V6(Ipv6Addr::from(address_octets));
                Ok(NymNodeRoutingAddress::Node(SocketAddr::new(ip, port)))
            }
            AddressType::Client => {
                let mut address_bytes = [0u8; ClientAddress::LEN];
                address_bytes.copy_from_slice(&b[1..ClientAddress::LEN + 1]);
                Ok(NymNodeRoutingAddress::Client(ClientAddress::from_bytes(
                    address_bytes,
                )))
            }
        }
    }

    /// Variant tag of this routing address.
    pub fn addr_type(&self) -> AddressType {
        match self {
            NymNodeRoutingAddress::Node(SocketAddr::V4(_)) => AddressType::Ipv4,
            NymNodeRoutingAddress::Node(SocketAddr::V6(_)) => AddressType::Ipv6,
            NymNodeRoutingAddress::Client(_) => AddressType::Client,
        }
    }
}

impl From<SocketAddr> for NymNodeRoutingAddress {
    fn from(addr: SocketAddr) -> Self {
        NymNodeRoutingAddress::Node(addr)
    }
}

impl From<ClientAddress> for NymNodeRoutingAddress {
    fn from(addr: ClientAddress) -> Self {
        NymNodeRoutingAddress::Client(addr)
    }
}

impl TryInto<NodeAddressBytes> for NymNodeRoutingAddress {
    type Error = NymNodeRoutingAddressError;

    /// On-wire encoding of a `NymNodeRoutingAddress` as a fixed-size sphinx routing field.
    /// VARIANT_TAG || payload || zeropad
    ///   - 0x04: IPv4 socket — payload is `port (2) || octets (4)`
    ///   - 0x06: IPv6 socket — payload is `port (2) || octets (16)`
    ///   - 0x0C: client fingerprint — payload is `client_address (20)`
    fn try_into(self) -> Result<NodeAddressBytes, Self::Error> {
        // first check if we have enough bytes to represent `self`:
        if self.bytes_min_len() > NODE_ADDRESS_LENGTH {
            return Err(NymNodeRoutingAddressError::TooSmallBytesRepresentation {
                required: self.bytes_min_len(),
            });
        }

        let padded_address = self.as_zero_padded_bytes(NODE_ADDRESS_LENGTH);

        let mut node_address_bytes = [0u8; 32];
        node_address_bytes.copy_from_slice(&padded_address);

        Ok(NodeAddressBytes::from_bytes(node_address_bytes))
    }
}

impl TryFrom<NodeAddressBytes> for NymNodeRoutingAddress {
    type Error = NymNodeRoutingAddressError;

    fn try_from(value: NodeAddressBytes) -> Result<Self, Self::Error> {
        Self::try_from_bytes(value.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4() -> NymNodeRoutingAddress {
        NymNodeRoutingAddress::Node(SocketAddr::new(IpAddr::from([1, 2, 3, 4]), 42))
    }

    fn v6() -> NymNodeRoutingAddress {
        NymNodeRoutingAddress::Node(SocketAddr::new(
            IpAddr::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            42,
        ))
    }

    fn client() -> NymNodeRoutingAddress {
        NymNodeRoutingAddress::Client(ClientAddress::from_bytes([7u8; ClientAddress::LEN]))
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_v4_address() {
        let address = v4();
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_v6_address() {
        let address = v6();
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_client_address() {
        let address = client();
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_empty_v4_address() {
        let address = NymNodeRoutingAddress::Node(SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 42));
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_empty_v6_address() {
        let address = NymNodeRoutingAddress::Node(SocketAddr::new(
            IpAddr::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            42,
        ));
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_node_address_bytes_with_no_data_loss()
    {
        for address in [v4(), v6(), client()] {
            let node_address: NodeAddressBytes = address.try_into().unwrap();
            assert_eq!(address, node_address.try_into().unwrap());
        }
    }

    #[test]
    fn try_from_bytes_rejects_unknown_variant_tag() {
        let bytes = [0xFFu8, 0, 0, 0, 0, 0, 0];
        assert!(matches!(
            NymNodeRoutingAddress::try_from_bytes(&bytes),
            Err(NymNodeRoutingAddressError::InvalidAddressType { received: 0xFF })
        ));
    }
}
