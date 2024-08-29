// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Pagination {
    pub total: usize,
    pub page: u32,
    pub size: usize,
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PaginatedResponse<T> {
    pub pagination: Pagination,
    pub data: Vec<T>,
}
