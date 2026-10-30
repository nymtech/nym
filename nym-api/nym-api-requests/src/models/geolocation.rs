// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::pagination::PaginatedResponse;
use nym_geolocation_contract_common::GeolocationRecord;
use serde::{Deserialize, Serialize};
use tendermint::block::Height;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeolocationEntriesRecordsResponse {
    /// The height every record was pinned to - the same height the client obtained a quorum'd
    /// snapshot for.
    #[schema(value_type = String)]
    pub height: Height,

    /// The location entries and the whitelisted agents alike, exactly as committed.
    pub entries: PaginatedResponse<GeolocationRecord>,
}
