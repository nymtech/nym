// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::WalletState;
use mixnet_contract_common::MixNodeConfigUpdate;
use mixnet_contract_common::{ExecuteMsg, Gateway, MixNode, NodeId};
use nym_types::currency::DecCoin;
use nym_types::mixnode::MixNodeCostParams;

async fn simulate_mixnet_operation(
    msg: ExecuteMsg,
    raw_funds: Option<DecCoin>,
    state: &WalletState,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let funds = if let Some(funds) = raw_funds
        .map(|c| guard.attempt_convert_to_base_coin(c))
        .transpose()?
    {
        vec![funds]
    } else {
        Vec::new()
    };

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client
        .nymd
        .wrap_contract_execute_message(mixnet_contract, &msg, funds)?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    owner_signature: String,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        },
        Some(pledge),
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_unbond_gateway(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::UnbondGateway {}, None, &state).await
}

#[tauri::command]
pub async fn simulate_bond_mixnode(
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    owner_signature: String,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let cost_params = cost_params.try_convert_to_mixnet_contract_cost_params(reg)?;

    simulate_mixnet_operation(
        ExecuteMsg::BondMixnode {
            mix_node: mixnode,
            cost_params,
            owner_signature,
        },
        Some(pledge),
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_unbond_mixnode(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::UnbondMixnode {}, None, &state).await
}

#[tauri::command]
pub async fn simulate_update_mixnode_cost_params(
    new_costs: MixNodeCostParams,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let new_costs = new_costs.try_convert_to_mixnet_contract_cost_params(reg)?;

    simulate_mixnet_operation(
        ExecuteMsg::UpdateMixnodeCostParams { new_costs },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_update_mixnode_config(
    update: MixNodeConfigUpdate,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(
        ExecuteMsg::UpdateMixnodeConfig { new_config: update },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_delegate_to_mixnode(
    mix_id: NodeId,
    amount: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(
        ExecuteMsg::DelegateToMixnode { mix_id },
        Some(amount),
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_undelegate_from_mixnode(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::UndelegateFromMixnode { mix_id }, None, &state).await
}

#[tauri::command]
pub async fn simulate_claim_operator_reward(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::WithdrawOperatorReward {}, None, &state).await
}

#[tauri::command]
pub async fn simulate_claim_delegator_reward(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::WithdrawDelegatorReward { mix_id }, None, &state).await
}
