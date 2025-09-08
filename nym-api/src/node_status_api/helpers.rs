// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::storage::NymApiStorage;
use nym_api_requests::models::{
    GatewayCoreStatusResponse, GatewayUptimeHistoryResponse, MixnodeCoreStatusResponse,
    MixnodeUptimeHistoryResponse,
};
use nym_mixnet_contract_common::NodeId;

pub(crate) async fn _gateway_uptime_history(
    storage: &NymApiStorage,
    identity: &str,
) -> AxumResult<GatewayUptimeHistoryResponse> {
    let history = storage
        .get_gateway_uptime_history_by_identity(identity)
        .await
        .map_err(AxumErrorResponse::not_found)?;

    Ok(GatewayUptimeHistoryResponse {
        identity: history.identity,
        history: history.history.into_iter().map(Into::into).collect(),
    })
}

pub(crate) async fn _gateway_core_status_count(
    storage: &NymApiStorage,
    identity: &str,
    since: Option<i64>,
) -> AxumResult<GatewayCoreStatusResponse> {
    let count = storage
        .get_core_gateway_status_count_by_identity(identity, since)
        .await
        .map_err(AxumErrorResponse::not_found)?;

    Ok(GatewayCoreStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

pub(crate) async fn _mixnode_uptime_history(
    storage: &NymApiStorage,
    mix_id: NodeId,
) -> AxumResult<MixnodeUptimeHistoryResponse> {
    let history = storage
        .get_mixnode_uptime_history(mix_id)
        .await
        .map_err(AxumErrorResponse::not_found)?;

    Ok(MixnodeUptimeHistoryResponse {
        mix_id,
        identity: history.identity,
        history: history.history.into_iter().map(Into::into).collect(),
    })
}

pub(crate) async fn _mixnode_core_status_count(
    storage: &NymApiStorage,
    mix_id: NodeId,
    since: Option<i64>,
) -> AxumResult<MixnodeCoreStatusResponse> {
    let count = storage
        .get_core_mixnode_status_count(mix_id, since)
        .await
        .map_err(AxumErrorResponse::not_found)?;

    Ok(MixnodeCoreStatusResponse { mix_id, count })
}
