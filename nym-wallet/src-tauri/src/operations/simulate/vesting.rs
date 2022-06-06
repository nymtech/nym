// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::simulate::{FeeDetails, SimulateResult};
use crate::State;
use mixnet_contract_common::IdentityKey;
use mixnet_contract_common::{Gateway, MixNode};
use std::sync::Arc;
use tokio::sync::RwLock;
use vesting_contract_common::ExecuteMsg;

#[tauri::command]
pub async fn simulate_vesting_bond_gateway(
    gateway: Gateway,
    pledge: Coin,
    owner_signature: String,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let pledge = pledge.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let vesting_contract = client.nymd.vesting_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        vesting_contract,
        &ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
            amount: pledge.into(),
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_unbond_gateway(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let vesting_contract = client.nymd.vesting_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        vesting_contract,
        &ExecuteMsg::UnbondGateway {},
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: Coin,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let pledge = pledge.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let vesting_contract = client.nymd.vesting_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        vesting_contract,
        &ExecuteMsg::BondMixnode {
            mix_node: mixnode,
            owner_signature,
            amount: pledge.into(),
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_unbond_mixnode(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let vesting_contract = client.nymd.vesting_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        vesting_contract,
        &ExecuteMsg::UnbondMixnode {},
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_update_mixnode(
    profit_margin_percent: u8,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let vesting_contract = client.nymd.vesting_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        vesting_contract,
        &ExecuteMsg::UpdateMixnodeConfig {
            profit_margin_percent,
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_withdraw_vested_coins(
    amount: Coin,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let amount = amount.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let vesting_contract = client.nymd.vesting_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        vesting_contract,
        &ExecuteMsg::WithdrawVestedCoins {
            amount: amount.into(),
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_claim_operator_reward(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let result = nymd_client!(state)
        .simulate_vesting_claim_operator_reward(None)
        .await?;
    let gas_price = nymd_client!(state).gas_price().clone();
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_compound_operator_reward(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let result = nymd_client!(state)
        .simulate_vesting_compound_operator_reward(None)
        .await?;
    let gas_price = nymd_client!(state).gas_price().clone();
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_claim_delegator_reward(
    mix_identity: IdentityKey,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let result = nymd_client!(state)
        .simulate_vesting_claim_delegator_reward(mix_identity, None)
        .await?;
    let gas_price = nymd_client!(state).gas_price().clone();
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_vesting_compound_delegator_reward(
    mix_identity: IdentityKey,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let result = nymd_client!(state)
        .simulate_vesting_compound_delegator_reward(mix_identity, None)
        .await?;
    let gas_price = nymd_client!(state).gas_price().clone();
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}
