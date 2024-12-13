// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::WalletState;
use nym_wallet_types::admin::TauriContractStateParams;

#[tauri::command]
pub async fn simulate_update_contract_settings(
    params: TauriContractStateParams,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let _ = params;
    let _ = state;
    Err(BackendError::Disabled)
}
