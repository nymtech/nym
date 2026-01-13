// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshMixnetContractCacheRequestBody {
    // for now no additional data is needed, but keep the struct for easier changes down the line
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshMixnetContractCacheResponse {
    pub success: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MixnetContractCacheTimestampResponse {
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshNodeStatusCacheRequestBody {
    // for now no additional data is needed, but keep the struct for easier changes down the line
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshNodeStatusCacheResponse {
    pub success: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeStatusCacheTimestampResponse {
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,
}
