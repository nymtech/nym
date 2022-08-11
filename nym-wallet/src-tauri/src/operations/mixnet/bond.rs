// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use crate::{Gateway, MixNode};
use mixnet_contract_common::MixNodeConfigUpdate;
use nym_types::currency::DecCoin;
use nym_types::gateway::GatewayBond;
use nym_types::mixnode::{MixNodeBond, MixNodeCostParams, MixNodeDetails};
use nym_types::transaction::TransactionExecuteResult;
use reqwest::Error as ReqwestError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use validator_client::nymd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nymd::{Coin, Fee};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDescription {
    name: String,
    description: String,
    link: String,
    location: String,
}

#[tauri::command]
pub async fn bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    owner_signature: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Bond gateway: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
        gateway.identity_key,
        pledge,
        pledge_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .bond_gateway(gateway, owner_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> Unbond gateway, fee = {:?}", fee);
    let res = guard.current_client()?.nymd.unbond_gateway(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn bond_mixnode(
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    owner_signature: String,
    pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let cost_params = cost_params.try_convert_to_mixnet_contract_cost_params(reg)?;
    log::info!(
        ">>> Bond mixnode: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
        mixnode.identity_key,
        pledge,
        pledge_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .bond_mixnode(mixnode, cost_params, owner_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> Unbond mixnode, fee = {:?}", fee);
    let res = guard.current_client()?.nymd.unbond_mixnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn update_mixnode_cost_params(
    new_costs: MixNodeCostParams,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let cost_params = new_costs.try_convert_to_mixnet_contract_cost_params(reg)?;
    log::info!(
        ">>> Update mixnode cost parameters: new parameters = {}, fee {:?}",
        cost_params.to_inline_json(),
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .update_mixnode_cost_params(cost_params, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn update_mixnode_config(
    update: MixNodeConfigUpdate,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Update mixnode config: update = {}, fee {:?}",
        update.to_inline_json(),
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .update_mixnode_config(update, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn mixnode_bond_details(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<MixNodeDetails>, BackendError> {
    log::info!(">>> Get mixnode bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let res = client.nymd.get_owned_mixnode(client.nymd.address()).await?;
    let details = res
        .mixnode_details
        .map(|details| {
            guard
                .registered_coins()
                .map(|reg| MixNodeDetails::from_mixnet_contract_mixnode_details(details, reg))
        })
        .transpose()?
        .transpose()?;
    log::info!(
        "<<< mix_id/identity_key = {:?}",
        details.as_ref().map(|r| (
            r.bond_information.id,
            &r.bond_information.mix_node.identity_key
        ))
    );
    log::trace!("<<< {:?}", details);
    Ok(details)
}

#[tauri::command]
pub async fn gateway_bond_details(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<GatewayBond>, BackendError> {
    log::info!(">>> Get gateway bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let bond = client.nymd.get_owned_gateway(client.nymd.address()).await?;
    let res = bond
        .gateway
        .map(|bond| {
            guard
                .registered_coins()
                .map(|reg| GatewayBond::from_mixnet_contract_gateway_bond(bond, reg))
        })
        .transpose()?
        .transpose()?;

    log::info!(
        "<<< identity_key = {:?}",
        res.as_ref().map(|r| r.gateway.identity_key.to_string())
    );
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_operator_rewards(
    address: String,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    todo!()
    // log::info!(">>> Get operator rewards for {}", address);
    // let guard = state.read().await;
    // let network = guard.current_network();
    // let denom = network.base_mix_denom();
    // let reward_amount = guard
    //     .current_client()?
    //     .nymd
    //     .get_operator_rewards(address)
    //     .await?;
    // let base_coin = Coin::new(reward_amount.u128(), denom);
    // let display_coin: DecCoin = guard.attempt_convert_to_display_dec_coin(base_coin.clone())?;
    // log::info!(
    //     "<<< rewards_base = {}, rewards_display = {}",
    //     base_coin,
    //     display_coin
    // );
    // Ok(display_coin)
}

#[tauri::command]
pub async fn get_number_of_mixnode_delegators(
    identity: String,
    state: tauri::State<'_, WalletState>,
) -> Result<usize, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let client = guard.current_client()?;
    // let paged_delegations = client
    //     .nymd
    //     .get_mix_delegations_paged(identity, None, Some(20))
    //     .await?;
    //
    // Ok(paged_delegations.delegations.len())
}

async fn fetch_mix_node_description(
    host: &str,
    port: u16,
) -> Result<NodeDescription, ReqwestError> {
    todo!()
    // let milli_second = Duration::from_millis(1000);
    // let client = reqwest::Client::builder().timeout(milli_second).build()?;
    // let response = client
    //     .get(format!("http://{}:{}/description", host, port))
    //     .send()
    //     .await;
    //
    // match response {
    //     Ok(res) => {
    //         let json = res.json::<NodeDescription>().await;
    //         match json {
    //             Ok(json) => Ok(json),
    //             Err(e) => Err(e),
    //         }
    //     }
    //     Err(e) => Err(e),
    // }
}

#[tauri::command]
pub async fn get_mix_node_description(
    host: &str,
    port: u16,
) -> Result<NodeDescription, BackendError> {
    return fetch_mix_node_description(host, port)
        .await
        .map_err(|e| BackendError::ReqwestError { source: e });
}
