// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::helpers::PaginationRequest;
use crate::unstable_routes::v1;
use nym_api_requests::nym_nodes::NodeRoleQueryParam;
use nym_http_api_common::Output;
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub(crate) struct NodesParamsWithRole {
    #[param(inline)]
    pub(crate) role: Option<NodeRoleQueryParam>,

    #[allow(dead_code)]
    pub(crate) semver_compatibility: Option<String>,
    pub(crate) no_legacy: Option<bool>,
    pub(crate) page: Option<u32>,
    pub(crate) per_page: Option<u32>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    pub(crate) epoch_id: Option<u32>,

    pub(crate) output: Option<Output>,
}

impl From<v1::nym_nodes::helpers::NodesParamsWithRole> for NodesParamsWithRole {
    fn from(value: v1::nym_nodes::helpers::NodesParamsWithRole) -> Self {
        NodesParamsWithRole {
            role: value.role,
            semver_compatibility: value.semver_compatibility,
            no_legacy: value.no_legacy,
            page: value.page,
            per_page: value.per_page,
            epoch_id: value.epoch_id,
            output: value.output,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct NodesParams {
    #[allow(dead_code)]
    pub(crate) semver_compatibility: Option<String>,
    pub(crate) no_legacy: Option<bool>,
    pub(crate) page: Option<u32>,
    pub(crate) per_page: Option<u32>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    pub(crate) epoch_id: Option<u32>,
    pub(crate) output: Option<Output>,
}

impl From<v1::nym_nodes::helpers::NodesParams> for NodesParams {
    fn from(value: v1::nym_nodes::helpers::NodesParams) -> Self {
        NodesParams {
            semver_compatibility: value.semver_compatibility,
            no_legacy: value.no_legacy,
            page: value.page,
            per_page: value.per_page,
            epoch_id: value.epoch_id,
            output: value.output,
        }
    }
}

impl From<NodesParamsWithRole> for NodesParams {
    fn from(params: NodesParamsWithRole) -> Self {
        NodesParams {
            semver_compatibility: params.semver_compatibility,
            no_legacy: params.no_legacy,
            page: params.page,
            per_page: params.per_page,
            epoch_id: params.epoch_id,
            output: params.output,
        }
    }
}

impl<'a> From<&'a NodesParams> for PaginationRequest {
    fn from(params: &'a NodesParams) -> Self {
        PaginationRequest {
            output: params.output,
            page: params.page,
            per_page: params.per_page,
        }
    }
}
