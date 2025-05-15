// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::helpers::PaginationRequest;
use nym_api_requests::models::{
    GatewayBondAnnotated, MalformedNodeBond, MixNodeBondAnnotated, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, NodeRoleQueryParam, SkimmedNode};
use nym_http_api_common::Output;
use nym_mixnet_contract_common::reward_params::Performance;
use serde::Deserialize;
use time::OffsetDateTime;

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

pub(crate) trait LegacyAnnotation {
    fn performance(&self) -> Performance;

    fn identity(&self) -> &str;

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond>;
}

impl LegacyAnnotation for MixNodeBondAnnotated {
    fn performance(&self) -> Performance {
        self.node_performance.last_24h
    }

    fn identity(&self) -> &str {
        self.identity_key()
    }

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        self.try_to_skimmed_node(role)
    }
}

impl LegacyAnnotation for GatewayBondAnnotated {
    fn performance(&self) -> Performance {
        self.node_performance.last_24h
    }

    fn identity(&self) -> &str {
        self.identity()
    }

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        self.try_to_skimmed_node(role)
    }
}

pub(crate) fn refreshed_at(
    iter: impl IntoIterator<Item = OffsetDateTime>,
) -> OffsetDateTimeJsonSchemaWrapper {
    iter.into_iter()
        .min()
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .into()
}
