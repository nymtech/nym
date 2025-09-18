// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api_client;
use crate::error::BackendError;
use crate::state::WalletState;
use cosmwasm_std::testing::mock_env;
use log::error;
use nym_mixnet_contract_common::reward_params::RewardedSetParams;
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{
    reward_params::Performance, Coin, IdentityKeyRef, Interval, IntervalRewardParams, NodeId,
    Percent, RewardingParams,
};
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::{
    AnnotationResponse, DisplayRole, GatewayCoreStatusResponse, GatewayStatusReportResponse,
    MixnodeCoreStatusResponse, MixnodeStatusResponse, StakeSaturationResponse,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct LegacyRewardEstimationResponse {
    estimation: RewardEstimate,
    reward_params: RewardingParams,
    epoch: Interval,
    as_at: i64,
}

impl LegacyRewardEstimationResponse {
    fn empty() -> LegacyRewardEstimationResponse {
        LegacyRewardEstimationResponse {
            estimation: Default::default(),
            reward_params: RewardingParams {
                interval: IntervalRewardParams {
                    reward_pool: Default::default(),
                    staking_supply: Default::default(),
                    staking_supply_scale_factor: Default::default(),
                    epoch_reward_budget: Default::default(),
                    stake_saturation_point: Default::default(),
                    sybil_resistance: Default::default(),
                    active_set_work_factor: Default::default(),
                    interval_pool_emission: Default::default(),
                },
                rewarded_set: RewardedSetParams {
                    entry_gateways: 0,
                    exit_gateways: 0,
                    mixnodes: 0,
                    standby: 0,
                },
            },
            epoch: Interval::init_interval(720, Duration::from_secs(60), &mock_env()),
            as_at: 0,
        }
    }
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn mixnode_core_node_status(
    mix_id: NodeId,
    since: Option<i64>,
    state: tauri::State<'_, WalletState>,
) -> Result<MixnodeCoreStatusResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_core_status_count(mix_id, since)
        .await?)
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn gateway_core_node_status(
    identity: IdentityKeyRef<'_>,
    since: Option<i64>,
    state: tauri::State<'_, WalletState>,
) -> Result<GatewayCoreStatusResponse, BackendError> {
    Ok(api_client!(state)
        .get_gateway_core_status_count(identity, since)
        .await?)
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn gateway_report(
    identity: IdentityKeyRef<'_>,
    _: tauri::State<'_, WalletState>,
) -> Result<GatewayStatusReportResponse, BackendError> {
    error!("‼️‼️‼️ using legacy and no longer supported gateway report query! returning a default response");
    Ok(GatewayStatusReportResponse {
        identity: identity.to_string(),
        owner: "".to_string(),
        most_recent: 0,
        last_hour: 0,
        last_day: 0,
    })
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn mixnode_status(
    _: NodeId,
    _: tauri::State<'_, WalletState>,
) -> Result<MixnodeStatusResponse, BackendError> {
    error!("‼️‼️‼️ using legacy and no longer supported mixnode status query! returning a default response");
    Ok(MixnodeStatusResponse {
        status: Default::default(),
    })
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn mixnode_reward_estimation(
    _: NodeId,
    _: tauri::State<'_, WalletState>,
) -> Result<LegacyRewardEstimationResponse, BackendError> {
    error!("‼️‼️‼️ using legacy and no longer supported mixnode reward estimation! returning a default response");
    Ok(LegacyRewardEstimationResponse::empty())
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn compute_mixnode_reward_estimation(
    _: u32,
    _: Option<Performance>,
    _: Option<u64>,
    _: Option<u64>,
    _: Option<Coin>,
    _: Option<Percent>,
    _: tauri::State<'_, WalletState>,
) -> Result<LegacyRewardEstimationResponse, BackendError> {
    error!("‼️‼️‼️ using legacy and no longer supported mixnode reward estimation! returning a default response");
    Ok(LegacyRewardEstimationResponse::empty())
}

// TODO: fix later (yeah...)
#[allow(deprecated)]
#[tauri::command]
pub async fn mixnode_stake_saturation(
    _: NodeId,
    _: tauri::State<'_, WalletState>,
) -> Result<StakeSaturationResponse, BackendError> {
    error!("‼️‼️‼️ using legacy and no longer supported mixnode stake saturation! returning a default response");
    Ok(StakeSaturationResponse {
        saturation: Default::default(),
        uncapped_saturation: Default::default(),
        as_at: 0,
    })
}

#[tauri::command]
pub async fn get_nymnode_role(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<DisplayRole>, BackendError> {
    let annotation = get_nymnode_annotation(node_id, state).await?;
    Ok(annotation.annotation.and_then(|n| n.current_role))
}

#[tauri::command]
pub async fn get_nymnode_annotation(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<AnnotationResponse, BackendError> {
    Ok(api_client!(state).get_node_annotation(node_id).await?)
}
