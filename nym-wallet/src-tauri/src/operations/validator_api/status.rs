// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api_client;
use crate::error::BackendError;
use crate::state::WalletState;
use validator_client::models::{
    DeprecatedRewardEstimationResponse, GatewayCoreStatusResponse, InclusionProbabilityResponse,
    MixnodeCoreStatusResponse, MixnodeStatusResponse, StakeSaturationResponse,
};

#[tauri::command]
pub async fn mixnode_core_node_status(
    identity: &str,
    since: Option<i64>,
    state: tauri::State<'_, WalletState>,
) -> Result<MixnodeCoreStatusResponse, BackendError> {
    todo!()
    // Ok(api_client!(state)
    //     .get_mixnode_core_status_count(identity, since)
    //     .await?)
}

#[tauri::command]
pub async fn gateway_core_node_status(
    identity: &str,
    since: Option<i64>,
    state: tauri::State<'_, WalletState>,
) -> Result<GatewayCoreStatusResponse, BackendError> {
    todo!()
    // Ok(api_client!(state)
    //     .get_gateway_core_status_count(identity, since)
    //     .await?)
}

#[tauri::command]
pub async fn mixnode_status(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<MixnodeStatusResponse, BackendError> {
    todo!()
    // Ok(api_client!(state).get_mixnode_status(identity).await?)
}

#[tauri::command]
pub async fn mixnode_reward_estimation(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<DeprecatedRewardEstimationResponse, BackendError> {
    todo!()
    // Ok(api_client!(state)
    //     .get_mixnode_reward_estimation(identity)
    //     .await?)
}

#[tauri::command]
pub async fn mixnode_stake_saturation(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<StakeSaturationResponse, BackendError> {
    todo!()
    // Ok(api_client!(state)
    //     .get_mixnode_stake_saturation(identity)
    //     .await?)
}

#[tauri::command]
pub async fn mixnode_inclusion_probability(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<InclusionProbabilityResponse, BackendError> {
    todo!()
    // Ok(api_client!(state)
    //     .get_mixnode_inclusion_probability(identity)
    //     .await?)
}
