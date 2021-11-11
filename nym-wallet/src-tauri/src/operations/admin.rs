use crate::format_err;
use crate::state::State;
use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use mixnet_contract::StateParams;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize)]
pub struct TauriStateParams {
  epoch_length: u32,
  minimum_mixnode_bond: String,
  minimum_gateway_bond: String,
  mixnode_bond_reward_rate: String,
  mixnode_delegation_reward_rate: String,
  mixnode_rewarded_set_size: u32,
  mixnode_active_set_size: u32,
}

impl From<StateParams> for TauriStateParams {
  fn from(p: StateParams) -> TauriStateParams {
    TauriStateParams {
      epoch_length: p.epoch_length,
      minimum_mixnode_bond: p.minimum_mixnode_bond.to_string(),
      minimum_gateway_bond: p.minimum_gateway_bond.to_string(),
      mixnode_bond_reward_rate: p.mixnode_bond_reward_rate.to_string(),
      mixnode_delegation_reward_rate: p.mixnode_delegation_reward_rate.to_string(),
      mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
      mixnode_active_set_size: p.mixnode_active_set_size,
    }
  }
}

impl TryFrom<TauriStateParams> for StateParams {
  type Error = Box<dyn std::error::Error>;

  fn try_from(p: TauriStateParams) -> Result<StateParams, Self::Error> {
    Ok(StateParams {
      epoch_length: p.epoch_length,
      minimum_mixnode_bond: Uint128::try_from(p.minimum_mixnode_bond.as_str())?,
      minimum_gateway_bond: Uint128::try_from(p.minimum_gateway_bond.as_str())?,
      mixnode_bond_reward_rate: Decimal::from_str(p.mixnode_bond_reward_rate.as_str())?,
      mixnode_delegation_reward_rate: Decimal::from_str(p.mixnode_delegation_reward_rate.as_str())?,
      mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
      mixnode_active_set_size: p.mixnode_active_set_size,
    })
  }
}

#[tauri::command]
pub async fn get_state_params(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriStateParams, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.get_state_params().await {
    Ok(params) => Ok(params.into()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn update_state_params(
  params: TauriStateParams,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriStateParams, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  let state_params: StateParams = match params.try_into() {
    Ok(state_params) => state_params,
    Err(e) => return Err(format_err!(e)),
  };
  match client.update_state_params(state_params.clone()).await {
    Ok(_) => Ok(state_params.into()),
    Err(e) => Err(format_err!(e)),
  }
}
