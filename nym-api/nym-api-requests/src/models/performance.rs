// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::pagination::PaginatedResponse;
use nym_performance_contract_common::LatestPerformance;
use serde::{Deserialize, Serialize};
use tendermint::block::Height;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceEntriesRecordsResponse {
    /// The height every entry was pinned to - the same height the client obtained a quorum'd
    /// snapshot for.
    #[schema(value_type = String)]
    pub height: Height,

    pub entries: PaginatedResponse<LatestPerformance>,
}
