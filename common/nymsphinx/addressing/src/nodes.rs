// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crypto::asymmetric::identity;
use nymsphinx_types::{NodeAddressBytes, NODE_ADDRESS_LENGTH};
use std::convert::{TryFrom, TryInto};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

// Not entirely sure whether this is the correct place for those, but let's see how it's going
// to work out
pub type NodeIdentity = identity::PublicKey;
pub const NODE_IDENTITY_SIZE: usize = identity::PUBLIC_KEY_LENGTH;

/// This module is responsible for encoding and decoding node routing information, so that
/// they could be later put into an appropriate field in a sphinx header.
/// Currently, that routing information is an IP address, but in principle it can be anything
/// for as long as it's going to fit in the field.

/// MAX_UNPADDED_LEN represents maximum length an unpadded address could have.
/// In this case it's an ipv6 socket address (with version prefix)
pub const MAX_NODE_ADDRESS_UNPADDED_LEN: usize = 19;

#[derive(Debug)]
pub enum NymNodeRoutingAddressError {
    InsufficientNumberOfBytesAvailableError,
    InvalidIpVersion,
}

/// Current representation of Node routing information used in Nym system.
/// At this point of time it is a simple `SocketAddr`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct NymNodeRoutingAddress(SocketAddr);

impl std::fmt::Display for NymNodeRoutingAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl NymNodeRoutingAddress {
    /// Minimum number of bytes that need to be available to represent self.
    /// The value has no upper bound as when converted into bytes, it's always
    /// padded with zeroes to be exactly NODE_ADDRESS_LENGTH long.
    pub fn bytes_min_len(&self) -> usize {
        match self.0 {
            SocketAddr::V4(_) => 7,
            SocketAddr::V6(_) => 19,
        }
    }

    /// Converts self into a vector of bytes.
    /// Note, this represents a generic bytes vector, not necessarily a NodeAddressBytes
    /// and hence is not zero-padded.
    pub fn as_bytes(&self) -> Vec<u8> {
        let port_bytes = self.0.port().to_be_bytes();
        let ip_octets_vec = match self.0.ip() {
            IpAddr::V4(ip) => ip.octets().to_vec(),
            IpAddr::V6(ip) => ip.octets().to_vec(),
        };

        std::iter::once(self.addr_type_as_u8())
            .chain(port_bytes.iter().cloned())
            .chain(ip_octets_vec.iter().cloned())
            .collect()
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
        // the bare minimum to represent `Self` is 7 bytes (for the shorter V4 version)
        if b.len() < 7 {
            return Err(NymNodeRoutingAddressError::InsufficientNumberOfBytesAvailableError);
        }

        let ip_version = b[0];
        let port: u16 = u16::from_be_bytes([b[1], b[2]]);
        let ip = match ip_version {
            4 => IpAddr::V4(Ipv4Addr::new(b[3], b[4], b[5], b[6])),
            6 => {
                if b.len() < 19 {
                    return Err(
                        NymNodeRoutingAddressError::InsufficientNumberOfBytesAvailableError,
                    );
                }
                let mut address_octets = [0u8; 16];
                address_octets.copy_from_slice(&b[3..19]);
                IpAddr::V6(Ipv6Addr::from(address_octets))
            }
            _ => return Err(NymNodeRoutingAddressError::InvalidIpVersion),
        };

        Ok(Self(SocketAddr::new(ip, port)))
    }

    /// Single byte representation of self ip version.
    pub fn addr_type_as_u8(&self) -> u8 {
        match self.0 {
            SocketAddr::V4(_) => 4,
            SocketAddr::V6(_) => 6,
        }
    }
}

/// Considering `NymNodeRoutingAddress` is equivalent to a `SocketAddr` at this point,
/// it makes perfect sense to allow the bilateral transformation.
impl From<SocketAddr> for NymNodeRoutingAddress {
    fn from(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

/// Considering `NymNodeRoutingAddress` is equivalent to a `SocketAddr` at this point,
/// it makes perfect sense to allow the bilateral transformation.
impl From<NymNodeRoutingAddress> for SocketAddr {
    fn from(addr: NymNodeRoutingAddress) -> Self {
        addr.0
    }
}

impl TryInto<NodeAddressBytes> for NymNodeRoutingAddress {
    type Error = NymNodeRoutingAddressError;

    /// `NymNodeRoutingAddress` (as a `SocketAddr`) is represented the following way:
    /// VersionFlag || port || octets || zeropad
    /// VersionFlag is one byte representing whether self is ipv4 or ipv6 address,
    /// port is 16bit big endian representation of port value
    /// octets is bytes representation of octets making up the ip address of the socket address
    /// (either 4 bytes for ipv4 or 16 bytes for ipv6)
    /// zeropad is padding of 0 for the `NymNodeRoutingAddress` to be
    /// exactly `NODE_ADDRESS_LENGTH` long.
    fn try_into(self) -> Result<NodeAddressBytes, Self::Error> {
        // first check if we have enough bytes to represent `self`:
        if self.bytes_min_len() > NODE_ADDRESS_LENGTH {
            return Err(NymNodeRoutingAddressError::InsufficientNumberOfBytesAvailableError);
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

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_v4_address() {
        let address = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([1, 2, 3, 4]), 42));
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_v6_address() {
        let address = NymNodeRoutingAddress(SocketAddr::new(
            IpAddr::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            42,
        ));
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_empty_v4_address() {
        let address = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 42));
        let address_bytes = address.as_bytes();
        assert_eq!(
            address,
            NymNodeRoutingAddress::try_from_bytes(&address_bytes).unwrap()
        )
    }

    #[test]
    fn nym_node_routing_address_can_be_converted_to_and_from_bytes_for_empty_v6_address() {
        let address = NymNodeRoutingAddress(SocketAddr::new(
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
        let address_v4 = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([1, 2, 3, 4]), 42));
        let address_v6 = NymNodeRoutingAddress(SocketAddr::new(
            IpAddr::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            42,
        ));

        let node_address1: NodeAddressBytes = address_v4.try_into().unwrap();
        let node_address2: NodeAddressBytes = address_v6.try_into().unwrap();

        assert_eq!(address_v4, node_address1.try_into().unwrap());
        assert_eq!(address_v6, node_address2.try_into().unwrap());
    }
}
