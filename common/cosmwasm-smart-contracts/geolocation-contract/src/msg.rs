// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ContractConfig;
use crate::constants::{DEFAULT_MAX_BATCH_SIZE, DEFAULT_MAX_PAYLOAD_SIZE, DEFAULT_MAX_SKEW_SECS};
use crate::types::{
    AgentPermissions, LocationPayload, Method, NymNodeLocation, RecordKey, Source, Subject,
};
#[cfg(feature = "schema")]
use crate::types::{
    AllRecordsPagedResponse, ConfigResponse, DigestResponse, EntryResponse, SubjectEntriesResponse,
    WhitelistResponse,
};
use cosmwasm_schema::cw_serde;
use nym_mixnet_contract_common::NodeId;

/// An agent whitelisted at instantiation, with the permissions it starts with.
#[cw_serde]
pub struct InitialAgent {
    pub agent: String,
    pub permissions: AgentPermissions,
}

/// One measurement in a batch. The measuring agent is the message sender rather than a field,
/// so an agent cannot write under another agent's key.
#[cw_serde]
pub struct Measurement {
    pub subject: Subject,
    pub method: Method,
    pub payload: LocationPayload,
}

/// Instantiate the geolocation contract. The instantiator becomes the admin.
#[cw_serde]
pub struct InstantiateMsg {
    /// Mixnet contract address, used to resolve node identity keys and to authorise the
    /// unbond callback.
    pub mixnet_contract_address: String,

    /// Agents authorised to write from the outset. May be empty, in which case the admin
    /// whitelists them afterwards.
    pub initial_whitelist: Vec<InitialAgent>,

    /// Overrides for the contract's tunables; each falls back to its `DEFAULT_` constant.
    pub max_skew_secs: Option<u64>,
    pub max_batch_size: Option<u32>,
    pub max_payload_size: Option<u32>,
}

impl InstantiateMsg {
    pub fn initial_contract_config(&self) -> ContractConfig {
        ContractConfig {
            max_skew_secs: self.max_skew_secs.unwrap_or(DEFAULT_MAX_SKEW_SECS),
            max_batch_size: self.max_batch_size.unwrap_or(DEFAULT_MAX_BATCH_SIZE),
            max_payload_size: self.max_payload_size.unwrap_or(DEFAULT_MAX_PAYLOAD_SIZE),
        }
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Submit a batch of measurements. Sender must be whitelisted with `can_measure`.
    ///
    /// All or nothing: one rejected entry fails the whole transaction. Ordering does not
    /// matter, since the accumulator is commutative, and the same key may appear more than
    /// once, in which case the last write stands.
    SubmitMeasurements { measurements: Vec<Measurement> },

    /// Relay a batch of node-signed self-declarations. Sender must be whitelisted with
    /// `can_relay_self_declared`.
    ///
    /// Kept separate from [`Self::SubmitMeasurements`] rather than merged into one batch: a
    /// relay carries data the agent did not produce and whose signature it cannot fully
    /// pre-validate against contract state, so one bad artifact must not be able to fail an
    /// agent's whole measurement sweep.
    RelaySelfDeclarations { declarations: Vec<NymNodeLocation> },

    /// Create or replace an override entry. Admin only.
    SetOverride {
        subject: Subject,
        payload: LocationPayload,
    },

    /// Delete an override entry. Admin only. Every other source for that subject is left
    /// untouched, so retracting an override does not wait on a re-measurement.
    RemoveOverride { subject: Subject },

    /// Add an agent to the whitelist, or change an existing agent's permissions. Admin only.
    SetWhitelistedAgent {
        agent: String,
        permissions: AgentPermissions,
    },

    /// Remove an agent from the whitelist. Admin only.
    ///
    /// Non-destructive: the agent's entries stay in storage and in the digest, and a
    /// conforming client stops honouring them immediately, because authorisation is evaluated
    /// against the current whitelist at read time. [`Self::PurgeAgentEntries`] cleans up
    /// afterwards, as hygiene rather than as the security control.
    RemoveWhitelistedAgent { agent: String },

    /// Delete up to `limit` entries written by a de-whitelisted agent, folding each removal
    /// into the digest. Admin only. Call repeatedly until nothing is left to purge.
    PurgeAgentEntries { agent: String, limit: Option<u32> },

    /// Transfer the admin role. Admin only. There is always exactly one admin.
    UpdateAdmin { admin: String },

    /// Change the contract's tunables. Admin only; omitted fields keep their current value.
    UpdateConfig {
        max_skew_secs: Option<u64>,
        max_batch_size: Option<u32>,
        max_payload_size: Option<u32>,
    },

    /// Cross-contract callback from the mixnet contract when `node_id` unbonds; deletes every
    /// entry for that node across all sources. Sender must be the configured mixnet contract.
    OnNymNodeUnbond { node_id: NodeId },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    #[cfg_attr(feature = "schema", returns(ConfigResponse))]
    Config {},

    /// A single entry, by its full key.
    #[cfg_attr(feature = "schema", returns(EntryResponse))]
    Entry { subject: Subject, source: Source },

    /// Every entry for one subject, across all sources. Unpaginated: a subject holds at most
    /// one entry per agent plus a self-declaration and an override.
    #[cfg_attr(feature = "schema", returns(SubjectEntriesResponse))]
    SubjectEntries { subject: Subject },

    /// Every entry for one bonded nym-node. The common case of [`Self::SubjectEntries`], saving
    /// callers that work in `NodeId`s from assembling a [`Subject`] themselves.
    #[cfg_attr(feature = "schema", returns(SubjectEntriesResponse))]
    NymNodeEntries { node_id: NodeId },

    /// Only the measured entries for one subject, dropping any self-declaration and override.
    #[cfg_attr(feature = "schema", returns(SubjectEntriesResponse))]
    SubjectMeasurements { subject: Subject },

    /// Paginated enumeration of every digest-committed record, across both entry classes.
    /// This is the global pull a client folds to recompute and verify the digest, so it has to
    /// cover the agent whitelist as well as the location entries.
    #[cfg_attr(feature = "schema", returns(AllRecordsPagedResponse))]
    AllRecords {
        start_after: Option<RecordKey>,
        limit: Option<u32>,
    },

    /// The 32-byte collapse of the accumulator. A convenience for comparing digests; it
    /// carries no proof, since smart queries cannot. Proving requires a raw store read at the
    /// documented digest key.
    #[cfg_attr(feature = "schema", returns(DigestResponse))]
    Digest {},

    /// The full agent whitelist. Unpaginated: the set is small and NYM-controlled.
    #[cfg_attr(feature = "schema", returns(WhitelistResponse))]
    Whitelist {},
}

/// Message passed to the contract's `migrate` entry point.
#[cw_serde]
pub struct MigrateMsg {}
