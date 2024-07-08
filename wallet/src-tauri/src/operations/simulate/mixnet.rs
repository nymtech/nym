// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::WalletState;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::{ExecuteMsg, Gateway, MixId, MixNode};
use nym_mixnet_contract_common::{GatewayConfigUpdate, MixNodeConfigUpdate};
use nym_types::currency::DecCoin;
use nym_types::mixnode::MixNodeCostParams;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;

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
    let mixnet_contract = client
        .nyxd
        .mixnet_contract_address()
        .expect("mixnet contract address is not available");

    let msg = client
        .nyxd
        .wrap_contract_execute_message(mixnet_contract, &msg, funds)?;

    let result = client.nyxd.simulate(vec![msg], "").await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    msg_signature: MessageSignature,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature: msg_signature,
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
    msg_signature: MessageSignature,
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
            owner_signature: msg_signature,
        },
        Some(pledge),
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_pledge_more(
    additional_pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::PledgeMore {}, Some(additional_pledge), &state).await
}

#[tauri::command]
pub async fn simulate_update_pledge(
    current_pledge: DecCoin,
    new_pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let dec_delta = guard.calculate_coin_delta(&current_pledge, &new_pledge)?;
    log::info!(
        ">>> Simulate pledge update, current pledge {}, new pledge {}",
        &current_pledge,
        &new_pledge,
    );

    match new_pledge.amount.cmp(&current_pledge.amount) {
        Ordering::Greater => {
            log::info!(
                "Simulate pledge increase, calculated additional pledge {}",
                dec_delta,
            );
            simulate_mixnet_operation(ExecuteMsg::PledgeMore {}, Some(dec_delta), &state).await
        }
        Ordering::Less => {
            log::info!(
                "Simulate pledge reduction, calculated reduction pledge {}",
                dec_delta,
            );
            simulate_mixnet_operation(
                ExecuteMsg::DecreasePledge {
                    decrease_by: guard.attempt_convert_to_base_coin(dec_delta)?.into(),
                },
                None,
                &state,
            )
            .await
        }
        Ordering::Equal => Err(BackendError::WalletPledgeUpdateNoOp),
    }
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
pub async fn simulate_update_gateway_config(
    update: GatewayConfigUpdate,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(
        ExecuteMsg::UpdateGatewayConfig { new_config: update },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_delegate_to_mixnode(
    mix_id: MixId,
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
    mix_id: MixId,
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
    mix_id: MixId,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_mixnet_operation(ExecuteMsg::WithdrawDelegatorReward { mix_id }, None, &state).await
}
