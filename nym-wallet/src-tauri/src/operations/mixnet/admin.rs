use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use mixnet_contract_common::ContractStateParams;
use nym_wallet_types::admin::TauriContractStateParams;
use std::convert::TryInto;
use validator_client::nymd::traits::MixnetQueryClient;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn get_contract_settings(
    state: tauri::State<'_, WalletState>,
) -> Result<TauriContractStateParams, BackendError> {
    log::info!(">>> Getting contract settings");

    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = &guard.current_client()?.nymd;

    let res = client.get_mixnet_contract_settings().await?;
    let converted = TauriContractStateParams::from_mixnet_contract_contract_state_params(res, reg)?;
    log::trace!("<<< {:?}", converted);
    Ok(converted)
}

#[tauri::command]
pub async fn update_contract_settings(
    params: TauriContractStateParams,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TauriContractStateParams, BackendError> {
    todo!()
    // let mixnet_contract_settings_params: ContractStateParams = params.try_into()?;
    // log::info!(
    //     ">>> Updating contract settings: {:?}",
    //     mixnet_contract_settings_params
    // );
    // nymd_client!(state)
    //     .update_contract_settings(mixnet_contract_settings_params.clone(), fee)
    //     .await?;
    // let res = mixnet_contract_settings_params.into();
    // log::trace!("<<< {:?}", res);
    // Ok(res)
}
