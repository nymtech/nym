// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::WalletState;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::{Gateway, MixNode, NodeId};
use nym_mixnet_contract_common::{GatewayConfigUpdate, MixNodeConfigUpdate};
use nym_types::currency::DecCoin;
use nym_types::mixnode::NodeCostParams;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_vesting_contract_common::ExecuteMsg;
use std::cmp::Ordering;

async fn simulate_vesting_operation(
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
    let vesting_contract = client
        .nyxd
        .vesting_contract_address()
        .expect("vesting contract address is not available");

    let msg = client
        .nyxd
        .wrap_contract_execute_message(vesting_contract, &msg, funds)?;

    let result = client.nyxd.simulate(vec![msg], "").await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_vesting_bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    msg_signature: MessageSignature,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let amount = guard.attempt_convert_to_base_coin(pledge)?.into();

    simulate_vesting_operation(
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature: msg_signature,
            amount,
        },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_vesting_unbond_gateway(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(ExecuteMsg::UnbondGateway {}, None, &state).await
}

#[tauri::command]
pub async fn simulate_vesting_bond_mixnode(
    mixnode: MixNode,
    cost_params: NodeCostParams,
    msg_signature: MessageSignature,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let cost_params = cost_params.try_convert_to_mixnet_contract_cost_params(reg)?;
    let amount = guard.attempt_convert_to_base_coin(pledge)?.into();

    simulate_vesting_operation(
        ExecuteMsg::BondMixnode {
            mix_node: mixnode,
            cost_params,
            owner_signature: msg_signature,
            amount,
        },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_vesting_update_pledge(
    current_pledge: DecCoin,
    new_pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    match new_pledge.amount.cmp(&current_pledge.amount) {
        Ordering::Greater => {
            let additional_pledge = guard
                .attempt_convert_to_base_coin(DecCoin {
                    amount: new_pledge.amount - current_pledge.amount,
                    denom: current_pledge.denom,
                })?
                .into();
            log::info!(
                ">>> Simulate pledge more, calculated additional pledge {}",
                additional_pledge,
            );
            simulate_vesting_operation(
                ExecuteMsg::PledgeMore {
                    amount: additional_pledge,
                },
                None,
                &state,
            )
            .await
        }
        Ordering::Less => {
            let decrease_pledge = guard
                .attempt_convert_to_base_coin(DecCoin {
                    amount: current_pledge.amount - new_pledge.amount,
                    denom: current_pledge.denom,
                })?
                .into();
            log::info!(
                ">>> Simulate decrease pledge, calculated decrease pledge {}",
                decrease_pledge,
            );
            simulate_vesting_operation(
                ExecuteMsg::DecreasePledge {
                    amount: decrease_pledge,
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
pub async fn simulate_vesting_pledge_more(
    additional_pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let amount = guard
        .attempt_convert_to_base_coin(additional_pledge)?
        .into();

    simulate_vesting_operation(ExecuteMsg::PledgeMore { amount }, None, &state).await
}

#[tauri::command]
pub async fn simulate_vesting_unbond_mixnode(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(ExecuteMsg::UnbondMixnode {}, None, &state).await
}

#[tauri::command]
pub async fn simulate_vesting_update_mixnode_cost_params(
    new_costs: NodeCostParams,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let new_costs = new_costs.try_convert_to_mixnet_contract_cost_params(reg)?;

    simulate_vesting_operation(
        ExecuteMsg::UpdateMixnodeCostParams { new_costs },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_vesting_update_mixnode_config(
    update: MixNodeConfigUpdate,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(
        ExecuteMsg::UpdateMixnodeConfig { new_config: update },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_vesting_update_gateway_config(
    update: GatewayConfigUpdate,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(
        ExecuteMsg::UpdateGatewayConfig { new_config: update },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_vesting_delegate_to_mixnode(
    mix_id: NodeId,
    amount: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let amount = guard.attempt_convert_to_base_coin(amount)?.into();

    simulate_vesting_operation(
        ExecuteMsg::DelegateToMixnode {
            on_behalf_of: None,
            mix_id,
            amount,
        },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_vesting_undelegate_from_mixnode(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(
        ExecuteMsg::UndelegateFromMixnode {
            on_behalf_of: None,
            mix_id,
        },
        None,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn simulate_withdraw_vested_coins(
    amount: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let amount = guard.attempt_convert_to_base_coin(amount)?.into();
    simulate_vesting_operation(ExecuteMsg::WithdrawVestedCoins { amount }, None, &state).await
}

#[tauri::command]
pub async fn simulate_vesting_claim_operator_reward(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(ExecuteMsg::ClaimOperatorReward {}, None, &state).await
}

#[tauri::command]
pub async fn simulate_vesting_claim_delegator_reward(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    simulate_vesting_operation(ExecuteMsg::ClaimDelegatorReward { mix_id }, None, &state).await
}
