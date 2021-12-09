use crate::format_err;
use crate::state::State;
use cosmwasm_std::Uint128;
use mixnet_contract::ContractStateParams;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize)]
pub struct TauriContractStateParams {
  minimum_mixnode_pledge: String,
  minimum_gateway_pledge: String,
  mixnode_rewarded_set_size: u32,
  mixnode_active_set_size: u32,
  active_set_work_factor: u8,
}

impl From<ContractStateParams> for TauriContractStateParams {
  fn from(p: ContractStateParams) -> TauriContractStateParams {
    TauriContractStateParams {
      minimum_mixnode_pledge: p.minimum_mixnode_pledge.to_string(),
      minimum_gateway_pledge: p.minimum_gateway_pledge.to_string(),
      mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
      mixnode_active_set_size: p.mixnode_active_set_size,
      active_set_work_factor: p.active_set_work_factor,
    }
  }
}

impl TryFrom<TauriContractStateParams> for ContractStateParams {
  type Error = Box<dyn std::error::Error>;

  fn try_from(p: TauriContractStateParams) -> Result<ContractStateParams, Self::Error> {
    Ok(ContractStateParams {
      minimum_mixnode_pledge: Uint128::try_from(p.minimum_mixnode_pledge.as_str())?,
      minimum_gateway_pledge: Uint128::try_from(p.minimum_gateway_pledge.as_str())?,
      mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
      mixnode_active_set_size: p.mixnode_active_set_size,
      active_set_work_factor: p.active_set_work_factor,
    })
  }
}

#[tauri::command]
pub async fn get_contract_settings(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractStateParams, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.get_contract_settings().await {
    Ok(params) => Ok(params.into()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn update_contract_settings(
  params: TauriContractStateParams,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractStateParams, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  let mixnet_contract_settings_params: ContractStateParams = match params.try_into() {
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
