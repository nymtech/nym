// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::simulate::FeeDetails;
use crate::WalletState;
use mixnet_contract_common::IdentityKey;
use mixnet_contract_common::{ExecuteMsg, Gateway, MixNode};
use nym_types::currency::DecCoin;

#[tauri::command]
pub async fn simulate_bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    owner_signature: String,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let pledge = guard.attempt_convert_to_base_coin(pledge)?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

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
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_unbond_gateway(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UnbondGateway {},
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let pledge = guard.attempt_convert_to_base_coin(pledge)?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::BondMixnode {
            mix_node: mixnode,
            owner_signature,
        },
        vec![pledge],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_unbond_mixnode(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UnbondMixnode {},
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_update_mixnode(
    profit_margin_percent: u8,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UpdateMixnodeConfig {
            profit_margin_percent,
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_delegate_to_mixnode(
    identity: &str,
    amount: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let delegation = guard.attempt_convert_to_base_coin(amount)?;

    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::DelegateToMixnode {
            mix_identity: identity.to_string(),
        },
        vec![delegation],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_undelegate_from_mixnode(
    identity: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let mixnet_contract = client.nymd.mixnet_contract_address();

    let msg = client.nymd.wrap_contract_execute_message(
        mixnet_contract,
        &ExecuteMsg::UndelegateFromMixnode {
            mix_identity: identity.to_string(),
        },
        vec![],
    )?;

    let result = client.nymd.simulate(vec![msg]).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_claim_operator_reward(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;

    let result = client.nymd.simulate_claim_operator_reward(None).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_compound_operator_reward(
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;

    let result = client.nymd.simulate_compound_operator_reward(None).await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_claim_delegator_reward(
    mix_identity: IdentityKey,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;

    let result = client
        .nymd
        .simulate_claim_delegator_reward(mix_identity, None)
        .await?;
    guard.create_detailed_fee(result)
}

#[tauri::command]
pub async fn simulate_compound_delegator_reward(
    mix_identity: IdentityKey,
    state: tauri::State<'_, WalletState>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;

    let result = client
        .nymd
        .simulate_compound_delegator_reward(mix_identity, None)
        .await?;
    guard.create_detailed_fee(result)
}
