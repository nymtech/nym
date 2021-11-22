use crate::format_err;
use crate::state::State;
use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use mixnet_contract::ContractSettingsParams;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize)]
pub struct TauriContractSettingsParams {
  epoch_length: u32,
  minimum_mixnode_bond: String,
  minimum_gateway_bond: String,
  mixnode_bond_reward_rate: String,
  mixnode_delegation_reward_rate: String,
  mixnode_rewarded_set_size: u32,
  mixnode_active_set_size: u32,
}

impl From<ContractSettingsParams> for TauriContractSettingsParams {
  fn from(p: ContractSettingsParams) -> TauriContractSettingsParams {
    TauriContractSettingsParams {
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

impl TryFrom<TauriContractSettingsParams> for ContractSettingsParams {
  type Error = Box<dyn std::error::Error>;

  fn try_from(p: TauriContractSettingsParams) -> Result<ContractSettingsParams, Self::Error> {
    Ok(ContractSettingsParams {
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
pub async fn get_contract_settings(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractSettingsParams, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.get_contract_settings().await {
    Ok(params) => Ok(params.into()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn update_contract_settings(
  params: TauriContractSettingsParams,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractSettingsParams, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  let mixnet_contract_settings_params: ContractSettingsParams = match params.try_into() {
    Ok(mixnet_contract_settings_params) => mixnet_contract_settings_params,
    Err(e) => return Err(format_err!(e)),
  };
  match client
    .update_contract_settings(mixnet_contract_settings_params.clone())
    .await
  {
    Ok(_) => Ok(mixnet_contract_settings_params.into()),
    Err(e) => Err(format_err!(e)),
  }
}
