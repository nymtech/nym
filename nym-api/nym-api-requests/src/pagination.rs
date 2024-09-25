// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/Pagination.ts")
)]
pub struct Pagination {
    pub total: usize,
    pub page: u32,
    pub size: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/PaginatedResponse.ts"
    )
)]
pub struct PaginatedResponse<T> {
    pub pagination: Pagination,
    pub data: Vec<T>,
}
