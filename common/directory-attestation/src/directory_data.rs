// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The whole-directory transfer payload (see `design.md`): the full entry set at a single
//! height plus the node-identity map, shipped by a producer so a client can recompute the
//! on-chain accumulator and node-identities hash offline and check them against a trusted
//! (quorum'd) [`SignedDigestSnapshot`](crate::SignedDigestSnapshot).
//!
//! Unlike a [`DirectorySubset`](crate::DirectorySubset), this carries no signature and needs
//! no canonical codec: the commitment is the per-leaf `DirectoryEntryRecord::digest_leaf()`
//! accumulator (plus the node-identities hash), both of which live in the trusted snapshot
//! and are independent of the JSON wire encoding. Any transport tampering is caught when the
//! client's recompute fails to match the snapshot, so plain serde DTOs are sufficient.

use cosmrs::tendermint::block::Height;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::bs58_ed25519_pubkey;
pub use nym_directory_contract_common::DirectoryEntryRecord;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeAs, SerializeAs};
use std::collections::BTreeMap;

#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DirectorySnapshotData {
    /// The height every entry (and identity) was pinned to - the same height the client
    /// obtained a quorum'd snapshot for.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub height: Height,

    /// The full entry set (node + curated), the raw leaves the client feeds into
    /// `recompute_accumulator` to match the snapshot's proven accumulator.
    pub records: Vec<DirectoryEntryRecord>,

    /// The `NodeId -> ed25519 identity` mapping, hashed into the snapshot's
    /// `node_identities_hash` and used to attribute node-entry authorship.
    #[serde_as(as = "BTreeMap<_, Ed25519PubKeySerde>")]
    #[cfg_attr(feature = "utoipa", schema(value_type = std::collections::BTreeMap<u32, String>))]
    pub node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
}

struct Ed25519PubKeySerde;

impl SerializeAs<ed25519::PublicKey> for Ed25519PubKeySerde {
    fn serialize_as<S>(value: &ed25519::PublicKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        bs58_ed25519_pubkey::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, ed25519::PublicKey> for Ed25519PubKeySerde {
    fn deserialize_as<D>(deserializer: D) -> Result<ed25519::PublicKey, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        bs58_ed25519_pubkey::deserialize(deserializer)
    }
}
