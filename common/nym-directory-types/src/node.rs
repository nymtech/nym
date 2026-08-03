// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::DirectoryTypesError;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// General self-reported information about a node, published under the directory.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeInformation {
    /// The current semver version of the nym-node binary.
    /// Allows the clients to determine the compatibility.
    #[prost(string, tag = "1")]
    pub binary_version: String,

    /// The node's announced hostname, if it has one.
    #[prost(string, optional, tag = "2")]
    pub hostname: Option<String>,

    /// The node's announced public IP addresses, string-encoded.
    #[prost(string, repeated, tag = "3")]
    pub ip_addresses: Vec<String>,

    /// The node's cosmos (nyx) account address, bech32-encoded.
    #[prost(string, tag = "4")]
    pub cosmos_address: String,

    /// Optional ISO 3166 alpha-2 country code of the node's physical location.
    #[prost(string, optional, tag = "5")]
    pub location: Option<String>,

    /// The node's externally-announced ports.
    #[prost(message, optional, tag = "6")]
    pub ports: Option<NodePorts>,

    /// The roles this node operates in.
    #[prost(message, optional, tag = "7")]
    pub modes: Option<NodeModes>,
}

/// A node's externally-announced ports. Stored as `u32` because protobuf has no 16-bit
/// integer type; use the accessors for the `u16` value.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodePorts {
    /// The verloc measurement port.
    #[prost(uint32, tag = "1")]
    pub verloc_port: u32,

    /// The mixnet (Sphinx) port.
    #[prost(uint32, tag = "2")]
    pub mix_port: u32,

    /// The unencrypted client websocket port.
    #[prost(uint32, tag = "3")]
    pub ws_port: u32,

    /// The encrypted client websocket (wss) port, if the node announces one.
    #[prost(uint32, optional, tag = "4")]
    pub wss_port: Option<u32>,
}

impl NodePorts {
    /// The verloc port as a `u16`.
    pub fn get_verloc_port(&self) -> u16 {
        self.verloc_port.min(u16::MAX as u32) as u16
    }

    /// The mixnet port as a `u16`.
    pub fn get_mix_port(&self) -> u16 {
        self.mix_port.min(u16::MAX as u32) as u16
    }

    /// The client websocket port as a `u16`.
    pub fn get_ws_port(&self) -> u16 {
        self.ws_port.min(u16::MAX as u32) as u16
    }

    /// The client wss port as a `u16`, if the node announces one.
    pub fn get_wss_port(&self) -> Option<u16> {
        self.wss_port.map(|port| port.min(u16::MAX as u32) as u16)
    }
}

/// The set of roles a node can operate in.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeModes {
    /// Specifies whether this node can operate in a mixnode mode.
    #[prost(bool, tag = "1")]
    pub mixnode: bool,

    /// Specifies whether this node can operate in an entry mode.
    #[prost(bool, tag = "2")]
    pub entry: bool,

    /// Specifies whether this node can operate in an exit mode.
    #[prost(bool, tag = "3")]
    pub exit: bool,

    /// Specifies whether this node has enabled wireguard.
    #[prost(bool, tag = "4")]
    pub wireguard_enabled: bool,
}

/// The node's Lewes Protocol (LP) connection details.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LewesProtocolDetails {
    /// The node's LP x25519 public key, encoded (32 bytes).
    #[prost(bytes, tag = "1")]
    pub x25519_public_key: Vec<u8>,

    /// Per-KEM key digests, keyed by KEM type name (e.g. `mlkem768`, `mceliece`); a client
    /// verifies the node's post-quantum encapsulation keys against these.
    #[prost(btree_map = "string, message", tag = "2")]
    pub kem_key_digests: BTreeMap<String, KemKeyDigests>,

    /// LP TCP control port (default 41264) for establishing LP sessions.
    #[prost(uint32, tag = "3")]
    pub control_port: u32,

    /// LP UDP data port (default 51264) for Sphinx packets wrapped in LP.
    #[prost(uint32, tag = "4")]
    pub data_port: u32,
}

impl LewesProtocolDetails {
    /// The LP x25519 public key as a fixed 32-byte array, or an error if the stored bytes are
    /// not exactly 32 long (the length of an x25519 public key).
    pub fn x25519_public_key_bytes(&self) -> Result<[u8; 32], DirectoryTypesError> {
        if self.x25519_public_key.len() != 32 {
            return Err(DirectoryTypesError::InvalidX25519KeyLength {
                got: self.x25519_public_key.len(),
            });
        }

        let mut k = [0u8; 32];
        // SAFETY: we just checked the length is 32 bytes
        k.copy_from_slice(&self.x25519_public_key);
        Ok(k)
    }

    /// The LP control port as a `u16`.
    pub fn get_control_port(&self) -> u16 {
        self.control_port.min(u16::MAX as u32) as u16
    }

    /// The LP data port as a `u16`.
    pub fn get_data_port(&self) -> u16 {
        self.data_port.min(u16::MAX as u32) as u16
    }
}

/// The key digests for a single KEM type, keyed by hash-function name. Nested inside
/// [`LewesProtocolDetails::kem_key_digests`] because protobuf maps cannot hold a map value.
#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct KemKeyDigests {
    /// hash-function name -> digest bytes
    #[prost(btree_map = "string, bytes", tag = "1")]
    pub digests: BTreeMap<String, Vec<u8>>,
}

/// A human-readable, operator-provided description of a node.
#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeDescription {
    /// moniker defines a human-readable name for the node.
    #[prost(string, tag = "1")]
    pub moniker: String,

    /// website defines an optional website link.
    #[prost(string, tag = "2")]
    pub website: String,

    /// security contact defines an optional email for security contact.
    #[prost(string, tag = "3")]
    pub security_contact: String,

    /// details define other optional details.
    #[prost(string, tag = "4")]
    pub details: String,
}

// These tests exercise the payload codec (prost) against `NodeDescription`, whose
// field set is stable, rather than the empty `SphinxKey`/`Wireguard` placeholders -
// so round-trip, determinism, and forward-compatibility are actually observable.
#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NodeDescription {
        NodeDescription {
            moniker: "node-1".to_string(),
            website: "https://nym.example".to_string(),
            security_contact: "security@nym.example".to_string(),
            details: "an example node".to_string(),
        }
    }

    #[test]
    fn round_trips_through_prost() {
        let original = sample();
        let decoded = NodeDescription::decode(original.encode_to_vec().as_slice())
            .expect("a payload must decode from the bytes it encoded to");

        assert_eq!(decoded.moniker, original.moniker);
        assert_eq!(decoded.website, original.website);
        assert_eq!(decoded.security_contact, original.security_contact);
        assert_eq!(decoded.details, original.details);
    }

    #[test]
    fn added_field_is_ignored_by_older_reader() {
        // A future payload version that appends a field under a fresh tag (5), keeping
        // the existing tags 1-4 unchanged - the forward-compatible way to grow a payload.
        #[derive(Clone, PartialEq, Message)]
        struct NodeDescriptionNext {
            #[prost(string, tag = "1")]
            moniker: String,
            #[prost(string, tag = "2")]
            website: String,
            #[prost(string, tag = "3")]
            security_contact: String,
            #[prost(string, tag = "4")]
            details: String,
            #[prost(string, tag = "5")]
            new_field: String,
        }

        let extended = NodeDescriptionNext {
            moniker: "node-1".to_string(),
            website: "https://nym.example".to_string(),
            security_contact: "security@nym.example".to_string(),
            details: "an example node".to_string(),
            new_field: "unknown-to-the-old-reader".to_string(),
        };

        // The older reader, which predates tag 5, still decodes and simply drops the
        // unknown field.
        let decoded = NodeDescription::decode(extended.encode_to_vec().as_slice())
            .expect("an older reader must tolerate an unknown appended field");

        assert_eq!(decoded.moniker, extended.moniker);
        assert_eq!(decoded.website, extended.website);
        assert_eq!(decoded.security_contact, extended.security_contact);
        assert_eq!(decoded.details, extended.details);
    }
}
