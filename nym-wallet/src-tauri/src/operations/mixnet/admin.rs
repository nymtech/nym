// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::Fee;
use nym_wallet_types::admin::TauriContractStateParams;

#[tauri::command]
pub async fn get_contract_settings(
    state: tauri::State<'_, WalletState>,
) -> Result<TauriContractStateParams, BackendError> {
    let _ = state;
    Err(BackendError::Disabled)
}

#[tauri::command]
pub async fn update_contract_settings(
    params: TauriContractStateParams,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let _ = params;
    let _ = fee;
    let _ = state;
    Err(BackendError::Disabled)
}
