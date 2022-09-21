// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api_client;
use crate::error::BackendError;
use crate::state::WalletState;
use mixnet_contract_common::{reward_params::Performance, IdentityKeyRef, NodeId};
use validator_client::models::{
    ComputeRewardEstParam, GatewayCoreStatusResponse, InclusionProbabilityResponse,
    MixnodeCoreStatusResponse, MixnodeStatusResponse, RewardEstimationResponse,
    StakeSaturationResponse,
};

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

#[tauri::command]
pub async fn mixnode_status(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<MixnodeStatusResponse, BackendError> {
    Ok(api_client!(state).get_mixnode_status(mix_id).await?)
}

#[tauri::command]
pub async fn mixnode_reward_estimation(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<RewardEstimationResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_reward_estimation(mix_id)
        .await?)
}

#[tauri::command]
pub async fn compute_mixnode_reward_estimation(
    identity: &str,
    performance: Option<Performance>,
    active_in_rewarded_set: Option<bool>,
    pledge_amount: Option<u64>,
    total_delegation: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<RewardEstimationResponse, BackendError> {
    let request_body = ComputeRewardEstParam {
        performance,
        active_in_rewarded_set,
        pledge_amount,
        total_delegation,
    };
    Ok(api_client!(state)
        .compute_mixnode_reward_estimation(identity, &request_body)
        .await?)
}

#[tauri::command]
pub async fn mixnode_stake_saturation(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<StakeSaturationResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_stake_saturation(mix_id)
        .await?)
}

#[tauri::command]
pub async fn mixnode_inclusion_probability(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<InclusionProbabilityResponse, BackendError> {
    Ok(api_client!(state)
        .get_mixnode_inclusion_probability(mix_id)
        .await?)
}
