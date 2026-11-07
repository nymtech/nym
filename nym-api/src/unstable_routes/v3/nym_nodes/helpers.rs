// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::helpers::PaginationRequestV2;
use nym_api_requests::nym_nodes::NodeRoleQueryParam;
use nym_http_api_common::OutputV2;
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub(crate) struct NodesParamsWithRole {
    #[param(inline)]
    pub(crate) role: Option<NodeRoleQueryParam>,

    pub(crate) semver_compatibility: Option<String>,
    pub(crate) page: Option<u32>,
    pub(crate) per_page: Option<u32>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    pub(crate) epoch_id: Option<u32>,

    pub(crate) output: Option<OutputV2>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct NodesParams {
    #[expect(dead_code)]
    pub(crate) semver_compatibility: Option<String>,
    pub(crate) page: Option<u32>,
    pub(crate) per_page: Option<u32>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    pub(crate) epoch_id: Option<u32>,
    pub(crate) output: Option<OutputV2>,
}

impl From<NodesParamsWithRole> for NodesParams {
    fn from(params: NodesParamsWithRole) -> Self {
        NodesParams {
            semver_compatibility: params.semver_compatibility,
            page: params.page,
            per_page: params.per_page,
            epoch_id: params.epoch_id,
            output: params.output,
        }
    }
}

impl<'a> From<&'a NodesParams> for PaginationRequestV2 {
    fn from(params: &'a NodesParams) -> Self {
        PaginationRequestV2 {
            output: params.output,
            page: params.page,
            per_page: params.per_page,
        }
    }
}
