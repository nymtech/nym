// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::helpers::{
    create_gateway_bonding_sign_payload, create_mixnode_bonding_sign_payload,
};
use crate::state::WalletState;
use nym_mixnet_contract_common::{Gateway, MixNode};
use nym_types::currency::DecCoin;
use nym_types::mixnode::MixNodeCostParams;

async fn mixnode_bonding_msg_payload(
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    pledge: DecCoin,
    vesting: bool,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let cost_params = cost_params.try_convert_to_mixnet_contract_cost_params(reg)?;
    log::info!(
        ">>> Bond mixnode bonding signature: identity_key = {}, pledge_display = {}, pledge_base = {}, vesting = {vesting}",
        mixnode.identity_key,
        pledge,
        pledge_base,
    );

    let client = guard.current_client()?;

    // TODO: decide on exact structure here. Json? base58? some hash?
    // to be determined
    let msg =
        create_mixnode_bonding_sign_payload(client, mixnode, cost_params, pledge_base, vesting)
            .await?;
    Ok(msg.to_base58_string()?)
}

async fn gateway_bonding_msg_payload(
    gateway: Gateway,
    pledge: DecCoin,
    vesting: bool,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    let guard = state.read().await;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    log::info!(
        ">>> Bond gateway bonding signature: identity_key = {}, pledge_display = {}, pledge_base = {}, vesting = {vesting}",
        gateway.identity_key,
        pledge,
        pledge_base,
    );

    let client = guard.current_client()?;

    // TODO: decide on exact structure here. Json? base58? some hash?
    // to be determined
    let msg = create_gateway_bonding_sign_payload(client, gateway, pledge_base, vesting).await?;
    Ok(msg.to_base58_string()?)
}

#[tauri::command]
pub async fn generate_mixnode_bonding_msg_payload(
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    mixnode_bonding_msg_payload(mixnode, cost_params, pledge, false, state).await
}

#[tauri::command]
pub async fn vesting_generate_mixnode_bonding_msg_payload(
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    mixnode_bonding_msg_payload(mixnode, cost_params, pledge, true, state).await
}

#[tauri::command]
pub async fn generate_gateway_bonding_msg_payload(
    gateway: Gateway,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    gateway_bonding_msg_payload(gateway, pledge, false, state).await
}

#[tauri::command]
pub async fn vesting_generate_gateway_bonding_msg_payload(
    gateway: Gateway,
    pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    gateway_bonding_msg_payload(gateway, pledge, true, state).await
}
