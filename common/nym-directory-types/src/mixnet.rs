// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::DirectoryTypesError;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A node's rotation-tagged sphinx public keys, keyed by key-rotation id. Holds one key
/// mid-rotation and two during an overlap/pre-announce window; a client selects the key for
/// the current rotation.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SphinxKeys {
    /// key rotation id -> sphinx (x25519) public key bytes
    #[prost(btree_map = "uint32, bytes", tag = "1")]
    pub keys: BTreeMap<u32, Vec<u8>>,
}

impl SphinxKeys {
    /// The sphinx public key for a given rotation as a fixed 32-byte array. `None` if no key
    /// is published for that rotation; `Err` if the stored bytes are not exactly 32 long (the
    /// length of an x25519 public key).
    pub fn key_for_rotation(
        &self,
        rotation_id: u32,
    ) -> Option<Result<[u8; 32], DirectoryTypesError>> {
        let key_bytes = self.keys.get(&rotation_id)?;

        if key_bytes.len() != 32 {
            return Some(Err(DirectoryTypesError::InvalidX25519KeyLength {
                got: key_bytes.len(),
            }));
        }

        let mut k = [0u8; 32];
        // SAFETY: we just checked the length is 32 bytes
        k.copy_from_slice(key_bytes);
        Some(Ok(k))
    }
}

/// The mixnet service providers a node exposes. Each is optional - a node advertises only the
/// providers it actually runs.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MixnetServiceProviders {
    /// The node's network requester, if it runs one.
    #[prost(message, optional, tag = "1")]
    pub network_requester: Option<NetworkRequester>,

    /// The node's internet packet router, if it runs one.
    #[prost(message, optional, tag = "2")]
    pub internet_packet_router: Option<InternetPacketRouter>,

    /// The node's authenticator, if it runs one.
    #[prost(message, optional, tag = "3")]
    pub authenticator: Option<Authenticator>,
}

/// A node's internet packet router service provider.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InternetPacketRouter {
    /// The provider's mixnet address as an encoded `Recipient` (96 bytes).
    #[prost(bytes, tag = "1")]
    pub address: Vec<u8>,
}

impl InternetPacketRouter {
    /// The provider's `Recipient` as a fixed 96-byte array, or an error if the stored bytes
    /// are not exactly 96 long.
    pub fn recipient_bytes(&self) -> Result<[u8; 96], DirectoryTypesError> {
        if self.address.len() != 96 {
            return Err(DirectoryTypesError::InvalidRecipientLength {
                got: self.address.len(),
            });
        }

        let mut r = [0u8; 96];
        // SAFETY: we just checked the length is 96 bytes
        r.copy_from_slice(&self.address);
        Ok(r)
    }
}

/// A node's network requester service provider.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NetworkRequester {
    /// The provider's mixnet address as an encoded `Recipient` (96 bytes).
    #[prost(bytes, tag = "1")]
    pub address: Vec<u8>,
}

impl NetworkRequester {
    /// The provider's `Recipient` as a fixed 96-byte array, or an error if the stored bytes
    /// are not exactly 96 long.
    pub fn recipient_bytes(&self) -> Result<[u8; 96], DirectoryTypesError> {
        if self.address.len() != 96 {
            return Err(DirectoryTypesError::InvalidRecipientLength {
                got: self.address.len(),
            });
        }

        let mut r = [0u8; 96];
        // SAFETY: we just checked the length is 96 bytes
        r.copy_from_slice(&self.address);
        Ok(r)
    }
}

/// A node's authenticator service provider.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Authenticator {
    /// The provider's mixnet address as an encoded `Recipient` (96 bytes).
    #[prost(bytes, tag = "1")]
    pub address: Vec<u8>,
}

impl Authenticator {
    /// The provider's `Recipient` as a fixed 96-byte array, or an error if the stored bytes
    /// are not exactly 96 long.
    pub fn recipient_bytes(&self) -> Result<[u8; 96], DirectoryTypesError> {
        if self.address.len() != 96 {
            return Err(DirectoryTypesError::InvalidRecipientLength {
                got: self.address.len(),
            });
        }

        let mut r = [0u8; 96];
        // SAFETY: we just checked the length is 96 bytes
        r.copy_from_slice(&self.address);
        Ok(r)
    }
}
