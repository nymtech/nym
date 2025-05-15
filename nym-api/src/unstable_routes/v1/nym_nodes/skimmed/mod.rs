// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::{PaginatedCachedNodesResponse, SkimmedNode};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::FormattedResponse;
use utoipa::ToSchema;

pub(crate) mod handlers;
pub(crate) mod helpers;

pub type PaginatedSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponse<SkimmedNode>>>;

pub(crate) use handlers::*;

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesResponse")]
pub struct PaginatedCachedNodesResponseSchema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SkimmedNode)]
    pub nodes: PaginatedResponse<SkimmedNode>,
}
