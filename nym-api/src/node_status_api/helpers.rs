// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::storage::NymApiStorage;
use crate::support::caching::Cache;
use crate::{MixnetContractCache, NodeStatusCache};
use nym_api_requests::models::{
    ComputeRewardEstParam, GatewayBondAnnotated, GatewayCoreStatusResponse,
    GatewayStatusReportResponse, GatewayUptimeHistoryResponse, GatewayUptimeResponse,
    MixNodeBondAnnotated, MixnodeCoreStatusResponse, MixnodeStatusReportResponse,
    MixnodeStatusResponse, MixnodeUptimeHistoryResponse, RewardEstimationResponse,
    StakeSaturationResponse, UptimeResponse,
};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::NodeId;

async fn gateway_identity_to_node_id(
    cache: &NodeStatusCache,
    identity: &str,
) -> AxumResult<NodeId> {
    let node_id = cache
        .map_identity_to_node_id(identity)
        .await
        .ok_or(AxumErrorResponse::not_found("gateway bond not found"))?;
    Ok(node_id)
}

async fn get_gateway_bond_annotated(
    cache: &NodeStatusCache,
    node_id: NodeId,
) -> AxumResult<GatewayBondAnnotated> {
    cache
        .gateway_annotated(node_id)
        .await
        .ok_or(AxumErrorResponse::not_found("gateway bond not found"))
}

async fn get_gateway_bond_annotated_by_identity(
    cache: &NodeStatusCache,
    identity: &str,
) -> AxumResult<GatewayBondAnnotated> {
    let node_id = gateway_identity_to_node_id(cache, identity).await?;
    get_gateway_bond_annotated(cache, node_id).await
}

async fn get_mixnode_bond_annotated(
    cache: &NodeStatusCache,
    mix_id: NodeId,
) -> AxumResult<MixNodeBondAnnotated> {
    cache
        .mixnode_annotated(mix_id)
        .await
        .ok_or(AxumErrorResponse::not_found("mixnode bond not found"))
}

pub(crate) async fn _gateway_report(
    cache: &NodeStatusCache,
    identity: &str,
) -> AxumResult<GatewayStatusReportResponse> {
    let gateway = get_gateway_bond_annotated_by_identity(cache, identity).await?;

    Ok(GatewayStatusReportResponse {
        identity: gateway.identity().to_owned(),
        owner: gateway.owner().to_string(),
        most_recent: gateway.node_performance.most_recent.round_to_integer(),
        last_hour: gateway.node_performance.last_hour.round_to_integer(),
        last_day: gateway.node_performance.last_24h.round_to_integer(),
    })
}

pub(crate) async fn _gateway_uptime_history(
    storage: &NymApiStorage,
    nym_contract_cache: &MixnetContractCache,
    identity: &str,
) -> AxumResult<GatewayUptimeHistoryResponse> {
    let history = storage
        .get_gateway_uptime_history_by_identity(identity)
        .await
        .map_err(AxumErrorResponse::not_found)?;

    let owner = nym_contract_cache
        .legacy_gateway_owner(history.node_id)
        .await
        .ok_or_else(|| AxumErrorResponse::not_found("could not determine gateway owner"))?;

    Ok(GatewayUptimeHistoryResponse {
        identity: history.identity,
        owner,
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

pub(crate) async fn _mixnode_report(
    cache: &NodeStatusCache,
    mix_id: NodeId,
) -> AxumResult<MixnodeStatusReportResponse> {
    let mixnode = get_mixnode_bond_annotated(cache, mix_id).await?;

    Ok(MixnodeStatusReportResponse {
        mix_id,
        identity: mixnode.identity_key().to_owned(),
        owner: mixnode.owner().to_string(),
        most_recent: mixnode.node_performance.most_recent.round_to_integer(),
        last_hour: mixnode.node_performance.last_hour.round_to_integer(),
        last_day: mixnode.node_performance.last_24h.round_to_integer(),
    })
}

pub(crate) async fn _mixnode_uptime_history(
    storage: &NymApiStorage,
    nym_contract_cache: &MixnetContractCache,
    mix_id: NodeId,
) -> AxumResult<MixnodeUptimeHistoryResponse> {
    let history = storage
        .get_mixnode_uptime_history(mix_id)
        .await
        .map_err(AxumErrorResponse::not_found)?;

    let owner = nym_contract_cache
        .legacy_gateway_owner(mix_id)
        .await
        .ok_or_else(|| AxumErrorResponse::not_found("could not determine mixnode owner"))?;

    Ok(MixnodeUptimeHistoryResponse {
        mix_id,
        identity: history.identity,
        owner,
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

pub(crate) async fn _get_mixnode_avg_uptime(
    cache: &NodeStatusCache,
    mix_id: NodeId,
) -> AxumResult<UptimeResponse> {
    let mixnode = get_mixnode_bond_annotated(cache, mix_id).await?;

    Ok(UptimeResponse {
        mix_id,
        avg_uptime: mixnode.node_performance.last_24h.round_to_integer(),
        node_performance: mixnode.node_performance,
    })
}

pub(crate) async fn _get_gateway_avg_uptime(
    cache: &NodeStatusCache,
    identity: &str,
) -> AxumResult<GatewayUptimeResponse> {
    let gateway = get_gateway_bond_annotated_by_identity(cache, identity).await?;

    Ok(GatewayUptimeResponse {
        identity: identity.to_string(),
        avg_uptime: gateway.node_performance.last_24h.round_to_integer(),
        node_performance: gateway.node_performance,
    })
}
