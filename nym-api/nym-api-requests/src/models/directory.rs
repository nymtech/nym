// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::pagination::PaginatedResponse;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::bs58_ed25519_pubkey;
use nym_directory_contract_common::DirectoryEntryRecord;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeAs, SerializeAs};
use std::collections::BTreeMap;
use tendermint::block::Height;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DirectoryEntriesRecordsResponse {
    /// The height every entry (and identity) was pinned to - the same height the client
    /// obtained a quorum'd snapshot for.
    #[schema(value_type = String)]
    pub height: Height,

    pub entries: PaginatedResponse<DirectoryEntryRecord>,
}

#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DirectoryEntriesIdentitiesResponse {
    /// The height every entry (and identity) was pinned to - the same height the client
    /// obtained a quorum'd snapshot for.
    #[schema(value_type = String)]
    pub height: Height,

    /// The `NodeId -> ed25519 identity` mapping, hashed into the snapshot's
    /// `node_identities_hash` and used to attribute node-entry authorship.
    #[serde_as(as = "BTreeMap<_, Ed25519PubKeySerde>")]
    #[schema(value_type = std::collections::BTreeMap<u32, String>)]
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
