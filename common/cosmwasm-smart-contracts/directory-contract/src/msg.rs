// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{EntryKey, LabelConfig};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use nym_mixnet_contract_common::NodeId;

#[cfg(feature = "schema")]
use crate::types::{
    AllEntriesPagedResponse, AllowedLabelsResponse, Config, CuratedEntriesPagedResponse,
    CuratedEntryResponse, DigestResponse, NodeEntriesResponse, NodeEntryResponse, SequenceResponse,
};

/// Defines initial label to be created on contract instantiation.
#[cw_serde]
pub struct InitialLabel {
    pub label: String,
    pub config: LabelConfig,
}

/// Instantiate the directory contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// Mixnet contract address, used to resolve node bonds and identity keys.
    pub mixnet_contract_address: String,

    /// Initial label whitelist with per-label size limits.
    pub initial_labels: Vec<InitialLabel>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create or replace a node entry. Authorised by an ed25519 `signature` from
    /// the node's identity key over [`crate::node_signing_payload`]; any account
    /// may relay the transaction. `sequence` must equal the node's expected next
    /// sequence (query [`QueryMsg::Sequence`]).
    SetNodeEntry {
        node_id: NodeId,
        label: String,
        data: Binary,
        sequence: u64,
        signature: Binary,
    },

    /// Delete a node entry. Same authorisation as [`Self::SetNodeEntry`].
    DeleteNodeEntry {
        node_id: NodeId,
        label: String,
        sequence: u64,
        signature: Binary,
    },

    /// Create or replace a curated entry. Admin only.
    SetCuratedEntry {
        id: String,
        label: String,
        data: Binary,
    },

    /// Delete a curated entry. Admin only.
    RemoveCuratedEntry { id: String, label: String },

    /// Add or update a whitelisted label and its `max_size`. Admin only;
    /// `max_size` must not exceed [`crate::constants::MAX_LABEL_SIZE_CEILING`].
    SetLabel { label: String, max_size: u32 },

    /// Remove a label from the whitelist. Non-destructive: existing entries under
    /// the label stay readable; only new writes are blocked. Admin only.
    RemoveLabel { label: String },

    /// Transfer or clear the admin role. Admin only.
    UpdateAdmin { admin: Option<String> },

    /// Cross-contract callback from the mixnet contract when `node_id` unbonds;
    /// deletes all of that node's entries. Sender must be the configured mixnet
    /// contract.
    OnNymNodeUnbond { node_id: NodeId },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    /// Contract configuration and current admin.
    #[cfg_attr(feature = "schema", returns(Config))]
    Config {},

    /// A single node entry.
    #[cfg_attr(feature = "schema", returns(NodeEntryResponse))]
    NodeEntry { node_id: NodeId, label: String },

    /// A single curated entry.
    #[cfg_attr(feature = "schema", returns(CuratedEntryResponse))]
    CuratedEntry { id: String, label: String },

    /// All entries for one node.
    #[cfg_attr(feature = "schema", returns(NodeEntriesResponse))]
    NodeEntries { node_id: NodeId },

    /// Paginated enumeration of all curated entries.
    #[cfg_attr(feature = "schema", returns(CuratedEntriesPagedResponse))]
    AllCuratedEntries {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },

    /// Paginated enumeration of ALL entries (both namespaces) - the global pull a
    /// client uses to recompute and verify the digest.
    #[cfg_attr(feature = "schema", returns(AllEntriesPagedResponse))]
    AllEntries {
        start_after: Option<EntryKey>,
        limit: Option<u32>,
    },

    /// The next sequence a node must sign with.
    #[cfg_attr(feature = "schema", returns(SequenceResponse))]
    Sequence { node_id: NodeId },

    /// The compact 32-byte global digest.
    #[cfg_attr(feature = "schema", returns(DigestResponse))]
    Digest {},

    /// The label whitelist with per-label sizes.
    #[cfg_attr(feature = "schema", returns(AllowedLabelsResponse))]
    AllowedLabels {},
}

/// Message passed to the contract's `migrate` entry point.
#[cw_serde]
pub struct MigrateMsg {}
