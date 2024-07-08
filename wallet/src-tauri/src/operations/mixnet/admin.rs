// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use nym_mixnet_contract_common::ContractStateParams;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use nym_validator_client::nyxd::Fee;
use nym_wallet_types::admin::TauriContractStateParams;

#[tauri::command]
pub async fn get_contract_settings(
    state: tauri::State<'_, WalletState>,
) -> Result<TauriContractStateParams, BackendError> {
    log::info!(">>> Getting contract settings");

    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = &guard.current_client()?.nyxd;

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
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = &guard.current_client()?.nyxd;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    let mixnet_contract_settings_params: ContractStateParams =
        params.try_convert_to_mixnet_contract_params(reg)?;
    log::info!(
        ">>> Updating contract settings: {:?}",
        mixnet_contract_settings_params
    );
    let res = client
        .update_contract_state_params(mixnet_contract_settings_params, fee)
        .await?;

    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}
