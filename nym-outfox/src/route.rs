// I took everything from sphinx to decouple the crates

use std::fmt::{self, Display, Formatter};

use libcrux_kem::{Algorithm, PublicKey};

use crate::error::OutfoxError;

pub const SECURITY_PARAMETER: usize = 16; // k in the Sphinx paper. Measured in bytes; 128 bits.
pub const DESTINATION_ADDRESS_LENGTH: usize = 2 * SECURITY_PARAMETER;
pub const IDENTIFIER_LENGTH: usize = SECURITY_PARAMETER;
pub const NODE_ADDRESS_LENGTH: usize = 2 * SECURITY_PARAMETER;
pub const DEFAULT_PAYLOAD_SIZE: usize = 1024;

// in paper I
pub type SURBIdentifier = [u8; IDENTIFIER_LENGTH];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    // address in theory could be changed to a vec<u8> as it does not need to be strictly DESTINATION_ADDRESS_LENGTH long
    // but cannot be longer than that (assuming longest possible route)
    pub address: DestinationAddressBytes,
    pub identifier: SURBIdentifier,
}

impl Destination {
    pub fn new(address: DestinationAddressBytes, identifier: SURBIdentifier) -> Self {
        Self {
            address,
            identifier,
        }
    }
}

// in paper nu
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Hash)]
pub struct NodeAddressBytes([u8; NODE_ADDRESS_LENGTH]);

impl NodeAddressBytes {
    pub fn as_base58_string(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    pub fn try_from_base58_string<S: Into<String>>(val: S) -> Result<Self, OutfoxError> {
        let decoded = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(e) => {
                return Err(OutfoxError::InvalidRouting(format!(
                    "failed to decode node address from b58 string: {:?}",
                    e
                )))
            }
        };

        if decoded.len() != NODE_ADDRESS_LENGTH {
            return Err(OutfoxError::InvalidRouting(
                format!("decoded node address has invalid length").into(),
            ));
        }

        let mut address_bytes = [0; NODE_ADDRESS_LENGTH];
        address_bytes.copy_from_slice(&decoded[..]);

        Ok(NodeAddressBytes(address_bytes))
    }

    pub fn try_from_byte_slice(b: &[u8]) -> Result<Self, OutfoxError> {
        if b.len() != NODE_ADDRESS_LENGTH {
            return Err(OutfoxError::InvalidRouting(
                format!("received bytes got invalid length").into(),
            ));
        }

        let mut address_bytes = [0; NODE_ADDRESS_LENGTH];
        address_bytes.copy_from_slice(b);

        Ok(NodeAddressBytes(address_bytes))
    }

    pub fn from_bytes(b: [u8; NODE_ADDRESS_LENGTH]) -> Self {
        NodeAddressBytes(b)
    }

    /// View this `NodeAddressBytes` as an array of bytes.
    pub fn as_bytes(&self) -> &[u8; NODE_ADDRESS_LENGTH] {
        &self.0
    }

    /// Convert this `NodeAddressBytes` to an array of bytes.
    pub fn to_bytes(&self) -> [u8; NODE_ADDRESS_LENGTH] {
        self.0
    }
}

impl Display for NodeAddressBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_base58_string().fmt(f)
    }
}

pub struct Node {
    pub kem: Algorithm,
    pub address: NodeAddressBytes,
    pub pub_key: PublicKey,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            kem: self.kem,
            address: self.address.clone(),
            pub_key: PublicKey::decode(self.kem, &self.pub_key.encode()).unwrap(),
        }
    }
}
impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("kem", &self.kem)
            .field("address", &self.address)
            .field("pub_key", &self.pub_key.encode())
            .finish()
    }
}

impl Node {
    pub fn new(kem: Algorithm, address: NodeAddressBytes, pub_key: PublicKey) -> Self {
        Self {
            kem,
            address,
            pub_key,
        }
    }
}

// in paper delta
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Hash)]
pub struct DestinationAddressBytes([u8; DESTINATION_ADDRESS_LENGTH]);

impl DestinationAddressBytes {
    pub fn as_base58_string(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    pub fn try_from_base58_string<S: Into<String>>(val: S) -> Result<Self, OutfoxError> {
        let decoded = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(e) => {
                return Err(OutfoxError::InvalidRouting(format!(
                    "failed to decode destination from b58 string: {:?}",
                    e
                ))
                .into())
            }
        };

        if decoded.len() != DESTINATION_ADDRESS_LENGTH {
            return Err(OutfoxError::InvalidRouting(
                format!("decoded destination address has invalid length",).into(),
            ));
        }

        let mut address_bytes = [0; DESTINATION_ADDRESS_LENGTH];
        address_bytes.copy_from_slice(&decoded[..]);

        Ok(DestinationAddressBytes(address_bytes))
    }

    pub fn from_bytes(b: [u8; DESTINATION_ADDRESS_LENGTH]) -> Self {
        DestinationAddressBytes(b)
    }

    pub fn try_from_byte_slice(b: &[u8]) -> Result<Self, OutfoxError> {
        if b.len() != DESTINATION_ADDRESS_LENGTH {
            return Err(
                OutfoxError::InvalidRouting(format!("received bytes got invalid length")).into(),
            );
        }

        let mut address_bytes = [0; DESTINATION_ADDRESS_LENGTH];
        address_bytes.copy_from_slice(b);

        Ok(DestinationAddressBytes(address_bytes))
    }

    /// View this `DestinationAddressBytes` as an array of bytes.
    pub fn as_bytes_ref(&self) -> &[u8; DESTINATION_ADDRESS_LENGTH] {
        &self.0
    }

    /// Convert this `DestinationAddressBytes` to an array of bytes.
    pub fn as_bytes(&self) -> [u8; DESTINATION_ADDRESS_LENGTH] {
        self.0
    }
}

impl Display for DestinationAddressBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_base58_string().fmt(f)
    }
}
