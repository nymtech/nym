// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::{PaginatedCachedNodesResponseV2, SkimmedNodeV2};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::FormattedResponse;
use utoipa::ToSchema;

pub(crate) mod handlers;
pub(crate) mod helpers;

pub type PaginatedSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SkimmedNodeV2>>>;

pub(crate) use handlers::*;

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesResponseV3")]
pub struct PaginatedCachedNodesResponseV3Schema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SkimmedNodeV2)]
    pub nodes: PaginatedResponse<SkimmedNodeV2>,
}
