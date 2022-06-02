// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coin::Coin;
use crate::error::BackendError;
use crate::simulate::{FeeDetails, SimulateResult};
use crate::State;
use mixnet_contract_common::{ExecuteMsg, Gateway, MixNode};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn simulate_bond_gateway(
    gateway: Gateway,
    pledge: Coin,
    owner_signature: String,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let pledge = pledge.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    // TODO: I'm still not 100% convinced whether this should be exposed here or handled somewhere else in the client code
    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        },
        vec![pledge],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_unbond_gateway(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UnbondGateway {},
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: Coin,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let pledge = pledge.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::BondMixnode {
            mix_node: mixnode,
            owner_signature,
        },
        vec![pledge],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_unbond_mixnode(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UnbondMixnode {},
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_update_mixnode(
    profit_margin_percent: u8,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UpdateMixnodeConfig {
            profit_margin_percent,
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_delegate_to_mixnode(
    identity: &str,
    amount: Coin,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let delegation = amount.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::DelegateToMixnode {
            mix_identity: identity.to_string(),
        },
        vec![delegation],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}

#[tauri::command]
pub async fn simulate_undelegate_from_mixnode(
    identity: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();
    let gas_price = client.nymd.gas_price().clone();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UndelegateFromMixnode {
            mix_identity: identity.to_string(),
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}
