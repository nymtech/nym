// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api_client;
use crate::error::BackendError;
use crate::state::WalletState;
use validator_client::models::{
    ComputeRewardEstParam, CoreNodeStatusResponse, InclusionProbabilityResponse,
    MixnodeStatusResponse, RewardEstimationResponse, StakeSaturationResponse,
};

#[tauri::command]
pub async fn mixnode_core_node_status(
    identity: &str,
    since: Option<i64>,
    state: tauri::State<'_, WalletState>,
) -> Result<CoreNodeStatusResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_core_status_count(identity, since)
        .await?)
}

#[tauri::command]
pub async fn gateway_core_node_status(
    identity: &str,
    since: Option<i64>,
    state: tauri::State<'_, WalletState>,
) -> Result<CoreNodeStatusResponse, BackendError> {
    Ok(api_client!(state)
        .get_gateway_core_status_count(identity, since)
        .await?)
}

#[tauri::command]
pub async fn mixnode_status(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<MixnodeStatusResponse, BackendError> {
    Ok(api_client!(state).get_mixnode_status(identity).await?)
}

#[tauri::command]
pub async fn mixnode_reward_estimation(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<RewardEstimationResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_reward_estimation(identity)
        .await?)
}

#[tauri::command]
pub async fn compute_mixnode_reward_estimation(
    identity: &str,
    uptime: Option<u8>,
    is_active: Option<bool>,
    pledge_amount: Option<u64>,
    total_delegation: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<RewardEstimationResponse, BackendError> {
    let request_body = ComputeRewardEstParam {
        uptime,
        is_active,
        pledge_amount,
        total_delegation,
    };
    Ok(api_client!(state)
        .compute_mixnode_reward_estimation(identity, &request_body)
        .await?)
}

#[tauri::command]
pub async fn mixnode_stake_saturation(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<StakeSaturationResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_stake_saturation(identity)
        .await?)
}

#[tauri::command]
pub async fn mixnode_inclusion_probability(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<InclusionProbabilityResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_inclusion_probability(identity)
        .await?)
}
