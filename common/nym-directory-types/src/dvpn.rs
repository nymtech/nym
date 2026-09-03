// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::DirectoryTypesError;
use prost::Message;
use serde::{Deserialize, Serialize};

/// A node's wireguard connection details. Ports are stored as `u32` because protobuf has no
/// 16-bit integer type; use the accessors for the `u16` value.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Wireguard {
    /// The wireguard tunnel (data) port.
    #[prost(uint32, tag = "1")]
    pub tunnel_port: u32,

    /// The wireguard metadata port.
    #[prost(uint32, tag = "2")]
    pub metadata_port: u32,

    /// The node's wireguard x25519 public key, encoded (32 bytes).
    #[prost(bytes, tag = "3")]
    pub public_key: Vec<u8>,
}

impl Wireguard {
    /// The wireguard x25519 public key as a fixed 32-byte array, or an error if the stored
    /// bytes are not exactly 32 long (the length of an x25519 public key).
    pub fn public_key_bytes(&self) -> Result<[u8; 32], DirectoryTypesError> {
        if self.public_key.len() != 32 {
            return Err(DirectoryTypesError::InvalidX25519KeyLength {
                got: self.public_key.len(),
            });
        }

        let mut k = [0u8; 32];
        // SAFETY: we just checked the length is 32 bytes
        k.copy_from_slice(&self.public_key);
        Ok(k)
    }

    /// The tunnel port as a `u16`.
    pub fn get_tunnel_port(&self) -> u16 {
        self.tunnel_port.min(u16::MAX as u32) as u16
    }

    /// The metadata port as a `u16`.
    pub fn get_metadata_port(&self) -> u16 {
        self.metadata_port.min(u16::MAX as u32) as u16
    }
}
