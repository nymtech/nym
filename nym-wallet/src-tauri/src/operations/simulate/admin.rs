// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::simulate::detailed_fee;
use crate::State;
use mixnet_contract_common::{ContractStateParams, ExecuteMsg};
use nym_wallet_types::admin::TauriContractStateParams;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn simulate_update_contract_settings(
    params: TauriContractStateParams,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let mixnet_contract_settings_params: ContractStateParams = params.try_into()?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UpdateContractStateParams(mixnet_contract_settings_params),
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(detailed_fee(client, result))
}
