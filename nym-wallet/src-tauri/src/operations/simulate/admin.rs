// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::WalletState;
use nym_mixnet_contract_common::{ContractStateParams, ExecuteMsg};
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_wallet_types::admin::TauriContractStateParams;

#[tauri::command]
pub async fn simulate_update_contract_settings(
    params: TauriContractStateParams,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let mixnet_contract_settings_params: ContractStateParams =
        params.try_convert_to_mixnet_contract_params(reg)?;

    let client = guard.current_client()?;
    let mixnet_contract = client
        .nyxd
        .mixnet_contract_address()
        .expect("mixnet contract address is not available");

    let msg = client.nyxd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UpdateContractStateParams {
            updated_parameters: mixnet_contract_settings_params,
        },
        vec![],
    )?;

    let result = client.nyxd.simulate(vec![msg], "").await?;
    guard.create_detailed_fee(result)
}
