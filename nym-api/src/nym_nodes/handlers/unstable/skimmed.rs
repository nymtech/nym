// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::nym_nodes::handlers::unstable::SemverCompatibilityQueryParam;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::nym_nodes::{CachedNodesResponse, SkimmedNode};

//     - `/v1/unstable/nym-nodes/skimmed/active` - returns all Nym Nodes **AND** legacy mixnodes **AND** legacy gateways that are currently in the active set, unless `no-legacy` parameter is used
async fn nodes_basic_active(
    state: State<AppState>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/skimmed/standby` - returns all Nym Nodes **AND** legacy mixnodes **AND** legacy gateways that are currently in the standby set, unless `no-legacy` parameter is used
async fn nodes_basic_standby(
    state: State<AppState>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/mixnodes/skimmed/all` - returns all Nym Nodes **AND** legacy mixnodes that are currently bonded and support mixing role, unless `no-legacy` parameter is used
async fn mixnodes_basic_all(
    state: State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/mixnodes/skimmed/active` - returns all Nym Nodes **AND** legacy mixnodes that are currently in the active set, unless `no-legacy` parameter is used
async fn mixnodes_basic_active(
    state: State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/entry-gateways/skimmed/active` - returns all Nym Nodes **AND** legacy gateways that are currently in the active set and are assigned the entry role, unless `no-legacy` parameter is used
async fn entry_gateways_basic_active(
    state: State<AppState>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/exit-gateways/skimmed/active` - returns all Nym Nodes **AND** legacy gateways that are currently in the active set and are assigned the exit role, unless `no-legacy` parameter is used
async fn exit_gateways_basic_active(
    state: State<AppState>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/entry-gateways/skimmed/all` - returns all Nym Nodes **AND** legacy gateways that are currently bonded and support entry gateway role, unless `no-legacy` parameter is used
async fn entry_gateways_basic_all(
    state: State<AppState>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}

//     - `/v1/unstable/nym-nodes/exit-gateways/skimmed/all` - returns all Nym Nodes **AND** legacy gateways that are currently bonded and support exit gateway role, unless `no-legacy` parameter is used
async fn exit_gateways_basic_all(
    state: State<AppState>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    todo!()
}
