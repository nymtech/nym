use std::convert::TryInto;
use std::sync::Arc;

use tokio::sync::RwLock;

use mixnet_contract_common::ContractStateParams;
use nym_wallet_types::admin::TauriContractStateParams;
use validator_client::nymd::Fee;

use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;

#[tauri::command]
pub async fn get_contract_settings(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractStateParams, BackendError> {
  log::info!(">>> Getting contract settings");
  let res = nymd_client!(state).get_contract_settings().await?.into();
  log::trace!("<<< {:?}", res);
  Ok(res)
}

#[tauri::command]
pub async fn update_contract_settings(
  params: TauriContractStateParams,
  fee: Option<Fee>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractStateParams, BackendError> {
  let mixnet_contract_settings_params: ContractStateParams = params.try_into()?;
  log::info!(
    ">>> Updating contract settings: {:?}",
    mixnet_contract_settings_params
  );
  nymd_client!(state)
    .update_contract_settings(mixnet_contract_settings_params.clone(), fee)
    .await?;
  let res = mixnet_contract_settings_params.into();
  log::trace!("<<< {:?}", res);
  Ok(res)
}
