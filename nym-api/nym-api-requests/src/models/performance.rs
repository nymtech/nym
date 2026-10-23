// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::pagination::PaginatedResponse;
use nym_contracts_common::Percent;
use nym_performance_contract_common::{EpochId, KindMedians, LatestPerformance, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tendermint::block::Height;
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceEntriesRecordsResponse {
    /// The height every entry was pinned to - the same height the client obtained a quorum'd
    /// snapshot for.
    #[schema(value_type = String)]
    pub height: Height,

    pub entries: PaginatedResponse<LatestPerformance>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NodePerformanceResponseV3 {
    pub node_id: NodeId,

    /// Absent when the node has no committed entry in the latest settled snapshot's bond set.
    pub performance: Option<DetailedNodePerformanceV3>,
}

/// A node's committed performance entry, with its bundle's medians read beside it without proof.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DetailedNodePerformanceV3 {
    /// The committed score; absent when the contract could not score the bundle.
    #[schema(value_type = Option<String>)]
    pub performance_score: Option<Percent>,

    /// The performance contract epoch the score comes from.
    pub source_epoch: EpochId,

    /// The block time of the snapshot height the entry was read at.
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub computed_at: OffsetDateTime,

    /// The bundle's medians; absent for an entry older than the epoch before the newest.
    pub medians: Option<KindMedians>,
}
