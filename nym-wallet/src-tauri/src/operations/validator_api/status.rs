// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api_client;
use crate::error::BackendError;
use crate::state::State;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::models::{
  CoreNodeStatusResponse, InclusionProbabilityResponse, MixnodeStatusResponse,
  RewardEstimationResponse, StakeSaturationResponse,
};

#[tauri::command]
pub async fn mixnode_core_node_status(
  identity: &str,
  since: Option<i64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<CoreNodeStatusResponse, BackendError> {
  Ok(
    api_client!(state)
      .get_mixnode_core_status_count(identity, since)
      .await?,
  )
}

#[tauri::command]
pub async fn gateway_core_node_status(
  identity: &str,
  since: Option<i64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<CoreNodeStatusResponse, BackendError> {
  Ok(
    api_client!(state)
      .get_gateway_core_status_count(identity, since)
      .await?,
  )
}

#[tauri::command]
pub async fn mixnode_status(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MixnodeStatusResponse, BackendError> {
  Ok(api_client!(state).get_mixnode_status(identity).await?)
}

#[tauri::command]
pub async fn mixnode_reward_estimation(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<RewardEstimationResponse, BackendError> {
  Ok(
    api_client!(state)
      .get_mixnode_reward_estimation(identity)
      .await?,
  )
}

#[tauri::command]
pub async fn mixnode_stake_saturation(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<StakeSaturationResponse, BackendError> {
  Ok(
    api_client!(state)
      .get_mixnode_stake_saturation(identity)
      .await?,
  )
}

#[tauri::command]
pub async fn mixnode_inclusion_probability(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<InclusionProbabilityResponse, BackendError> {
  Ok(
    api_client!(state)
      .get_mixnode_inclusion_probability(identity)
      .await?,
  )
}
