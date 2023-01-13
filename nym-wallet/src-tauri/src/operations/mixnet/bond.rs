// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use crate::{nyxd_client, Gateway, MixNode};
use mixnet_contract_common::{MixId, MixNodeConfigUpdate};
use nym_types::currency::DecCoin;
use nym_types::gateway::GatewayBond;
use nym_types::mixnode::{MixNodeCostParams, MixNodeDetails};
use nym_types::transaction::TransactionExecuteResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use validator_client::nyxd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nyxd::Fee;

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
        .nyxd
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
    let res = guard.current_client()?.nyxd.unbond_gateway(fee).await?;
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
        .nyxd
        .bond_mixnode(mixnode, cost_params, owner_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn pledge_more(
    fee: Option<Fee>,
    additional_pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let additional_pledge_base = guard.attempt_convert_to_base_coin(additional_pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Pledge more, additional_pledge_display = {}, additional_pledge_base = {}, fee = {:?}",
        additional_pledge,
        additional_pledge_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .pledge_more(additional_pledge_base, fee)
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
    let res = guard.current_client()?.nyxd.unbond_mixnode(fee).await?;
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
        .nyxd
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
        .nyxd
        .update_mixnode_config(update, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn get_mixnode_avg_uptime(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<u8>, BackendError> {
    log::info!(">>> Get mixnode bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let res = client.nyxd.get_owned_mixnode(client.nyxd.address()).await?;

    match res.mixnode_details {
        Some(details) => {
            let id = details.mix_id();
            log::trace!("  >>> Get average uptime percentage: mix_id = {}", id);
            let avg_uptime_percent = client
                .nym_api
                .get_mixnode_avg_uptime(id)
                .await
                .ok()
                .map(|r| r.avg_uptime);
            log::trace!("  <<< {:?}", avg_uptime_percent);
            Ok(avg_uptime_percent)
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn mixnode_bond_details(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<MixNodeDetails>, BackendError> {
    log::info!(">>> Get mixnode bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let res = client.nyxd.get_owned_mixnode(client.nyxd.address()).await?;
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
            r.bond_information.mix_id,
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
    let bond = client.nyxd.get_owned_gateway(client.nyxd.address()).await?;
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
pub async fn get_pending_operator_rewards(
    address: String,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Get pending operator rewards for {}", address);
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_pending_operator_reward(&address.parse()?)
        .await?;

    // note to @MS: now we're able to obtain more information than just the pending reward
    // the entire returned struct contains the following:
    /*
       pub amount_staked: Option<Coin>,
       pub amount_earned: Option<Coin>,
       pub amount_earned_detailed: Option<Decimal>,
       pub mixnode_still_fully_bonded: bool,
    */

    let base_coin = res.amount_earned;
    let display_coin = base_coin
        .as_ref()
        .map(|c| guard.attempt_convert_to_display_dec_coin(c.clone().into()))
        .transpose()?
        .unwrap_or_else(|| guard.default_zero_mix_display_coin());

    log::info!(
        "<<< rewards_base = {:?}, rewards_display = {}",
        base_coin,
        display_coin
    );
    Ok(display_coin)
}

#[tauri::command]
pub async fn get_number_of_mixnode_delegators(
    mix_id: MixId,
    state: tauri::State<'_, WalletState>,
) -> Result<usize, BackendError> {
    Ok(nyxd_client!(state)
        .get_mixnode_details(mix_id)
        .await?
        .mixnode_details
        .map(|details| details.rewarding_details.unique_delegations)
        .unwrap_or_default() as usize)
}

#[tauri::command]
pub async fn get_mix_node_description(
    host: &str,
    port: u16,
) -> Result<NodeDescription, BackendError> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_millis(1000))
        .build()?
        .get(format!("http://{host}:{port}/description"))
        .send()
        .await?
        .json()
        .await?)
}

#[tauri::command]
pub async fn get_mixnode_uptime(
    mix_id: MixId,
    state: tauri::State<'_, WalletState>,
) -> Result<u8, BackendError> {
    log::info!(">>> Get mixnode uptime");

    let guard = state.read().await;
    let client = guard.current_client()?;
    let uptime = client.nym_api.get_mixnode_avg_uptime(mix_id).await?;

    log::info!(">>> Uptime response: {}", uptime.avg_uptime);
    Ok(uptime.avg_uptime)
}
