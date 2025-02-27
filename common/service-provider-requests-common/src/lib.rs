// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("invalid version: {0}")]
    InvalidVersion(u8),

    #[error("invalid service provider type: {0}")]
    InvalidServiceProviderType(u8),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ServiceProviderType {
    NetworkRequester = 0,
    IpPacketRouter = 1,
    Authenticator = 2,
}

impl fmt::Display for ServiceProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkRequester => write!(f, "NetworkRequester"),
            Self::IpPacketRouter => write!(f, "IpPacketRouter"),
            Self::Authenticator => write!(f, "Authenticator"),
        }
    }
}

impl TryFrom<u8> for ServiceProviderType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NetworkRequester),
            1 => Ok(Self::IpPacketRouter),
            2 => Ok(Self::Authenticator),
            _ => Err(ProtocolError::InvalidServiceProviderType(value)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Protocol {
    pub version: u8,
    pub service_provider_type: ServiceProviderType,
}

impl TryFrom<&[u8; 2]> for Protocol {
    type Error = ProtocolError;

    fn try_from(bytes: &[u8; 2]) -> Result<Self, Self::Error> {
        let version = bytes[0];
        let service_provider_type = ServiceProviderType::try_from(bytes[1])
            .map_err(|_| ProtocolError::InvalidServiceProviderType(bytes[1]))?;

        Ok(Self {
            version,
            service_provider_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::Options;

    fn make_bincode_serializer() -> impl bincode::Options {
        bincode::DefaultOptions::new()
            .with_big_endian()
            .with_varint_encoding()
    }

    #[test]
    fn protocol_serialization() {
        let protocol = Protocol {
            version: 4,
            service_provider_type: ServiceProviderType::NetworkRequester,
        };

        let serialized = make_bincode_serializer().serialize(&protocol).unwrap();
        let deserialized: Protocol = make_bincode_serializer().deserialize(&serialized).unwrap();

        assert_eq!(protocol, deserialized);
    }

    #[test]
    fn compact_serialization() {
        let protocol = Protocol {
            version: 4,
            service_provider_type: ServiceProviderType::NetworkRequester,
        };

        let serialized = make_bincode_serializer().serialize(&protocol).unwrap();
        assert_eq!(serialized.len(), 2);
    }

    #[test]
    fn protocol_deserialization() {
        let bytes = [4, ServiceProviderType::NetworkRequester as u8];
        let deserialized = Protocol::try_from(&bytes).unwrap();

        let expected = Protocol {
            version: 4,
            service_provider_type: ServiceProviderType::NetworkRequester,
        };

        assert_eq!(expected, deserialized);
    }

    #[test]
    fn invalid_protocol_deserialization() {
        let bytes = [4, 3];
        let deserialized = Protocol::try_from(&bytes);

        assert!(deserialized.is_err());
    }
}
