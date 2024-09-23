// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::operations::helpers::{
    verify_gateway_bonding_sign_payload, verify_mixnode_bonding_sign_payload,
    verify_nym_node_bonding_sign_payload,
};
use crate::state::WalletState;
use crate::{nyxd_client, Gateway, MixNode};
use log::info;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::gateway::GatewayConfigUpdate;
use nym_mixnet_contract_common::nym_node::{NodeConfigUpdate, StakeSaturationResponse};
use nym_mixnet_contract_common::{MixNodeConfigUpdate, NodeId, NymNode};
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_node_requests::api::ErrorResponse;
use nym_types::currency::DecCoin;
use nym_types::gateway::GatewayBond;
use nym_types::mixnode::{MixNodeDetails, NodeCostParams};
use nym_types::nym_node::NymNodeDetails;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use nym_validator_client::nyxd::Fee;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyNodeDescription {
    name: String,
    description: String,
    link: String,
    location: String,
}

#[tauri::command]
pub async fn bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    msg_signature: MessageSignature,
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

    let client = guard.current_client()?;
    // check the signature to make sure the user copied it correctly
    if let Err(err) =
        verify_gateway_bonding_sign_payload(client, &gateway, &pledge_base, false, &msg_signature)
            .await
    {
        log::warn!("failed to verify provided gateway bonding signature: {err}");
        return Err(err);
    }

    let res = client
        .nyxd
        .bond_gateway(gateway, msg_signature, pledge_base, fee)
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
    cost_params: NodeCostParams,
    msg_signature: MessageSignature,
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

    let client = guard.current_client()?;
    // check the signature to make sure the user copied it correctly
    if let Err(err) = verify_mixnode_bonding_sign_payload(
        client,
        &mixnode,
        &cost_params,
        &pledge_base,
        false,
        &msg_signature,
    )
    .await
    {
        log::warn!("failed to verify provided mixnode bonding signature: {err}");
        return Err(err);
    }

    let res = client
        .nyxd
        .bond_mixnode(mixnode, cost_params, msg_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn bond_nymnode(
    nymnode: NymNode,
    cost_params: NodeCostParams,
    msg_signature: MessageSignature,
    pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.write().await;
    let reg = guard.registered_coins()?;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let cost_params = cost_params.try_convert_to_mixnet_contract_cost_params(reg)?;
    log::info!(
        ">>> Bond NymNode: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
        nymnode.identity_key,
        pledge,
        pledge_base,
        fee,
    );

    let client = guard.current_client()?;
    // check the signature to make sure the user copied it correctly
    if let Err(err) = verify_nym_node_bonding_sign_payload(
        client,
        &nymnode,
        &cost_params,
        &pledge_base,
        &msg_signature,
    )
    .await
    {
        log::warn!("failed to verify provided nymnode bonding signature: {err}");
        return Err(err);
    }

    let res = client
        .nyxd
        .bond_nymnode(nymnode, cost_params, msg_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn update_pledge(
    current_pledge: DecCoin,
    new_pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let dec_delta = guard.calculate_coin_delta(&current_pledge, &new_pledge)?;
    let delta = guard.attempt_convert_to_base_coin(dec_delta.clone())?;
    log::info!(
        ">>> Pledge update, current pledge {}, new pledge {}",
        &current_pledge,
        &new_pledge,
    );

    let res = match new_pledge.amount.cmp(&current_pledge.amount) {
        Ordering::Greater => {
            log::info!(
                "Pledge increase, calculated additional pledge {}, fee = {:?}",
                &dec_delta,
                fee,
            );
            guard.current_client()?.nyxd.pledge_more(delta, fee).await?
        }
        Ordering::Less => {
            log::info!(
                "Pledge reduction, calculated reduction pledge {}, fee = {:?}",
                &dec_delta,
                fee,
            );
            guard
                .current_client()?
                .nyxd
                .decrease_pledge(delta, fee)
                .await?
        }
        Ordering::Equal => return Err(BackendError::WalletPledgeUpdateNoOp),
    };

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
pub async fn decrease_pledge(
    fee: Option<Fee>,
    decrease_by: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let decrease_by_base = guard.attempt_convert_to_base_coin(decrease_by.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Decrease pledge, pledge_decrease_display = {}, pledge_decrease_base = {}, fee = {:?}",
        decrease_by,
        decrease_by_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .decrease_pledge(decrease_by_base, fee)
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
pub async fn unbond_nymnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.write().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> Unbond NymNode, fee = {fee:?}");
    let res = guard.current_client()?.nyxd.unbond_nymnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);

    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn update_mixnode_cost_params(
    new_costs: NodeCostParams,
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
        .update_cost_params(cost_params, fee)
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
pub async fn update_gateway_config(
    update: GatewayConfigUpdate,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Update gateway config: update = {}, fee {:?}",
        update.to_inline_json(),
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .update_gateway_config(update, fee)
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
    let res = client
        .nyxd
        .get_owned_mixnode(&client.nyxd.address())
        .await?;

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
    let res = client
        .nyxd
        .get_owned_mixnode(&client.nyxd.address())
        .await?;
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
    let bond = client
        .nyxd
        .get_owned_gateway(&client.nyxd.address())
        .await?;
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
pub async fn nym_node_bond_details(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<NymNodeDetails>, BackendError> {
    log::info!(">>> Get nym-node bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let res = client
        .nyxd
        .get_owned_nymnode(&client.nyxd.address())
        .await?;
    let details = res
        .details
        .map(|details| {
            guard
                .registered_coins()
                .map(|reg| NymNodeDetails::from_mixnet_contract_nym_node_details(details, reg))
        })
        .transpose()?
        .transpose()?;
    log::info!(
        "<<< node_id/identity_key = {:?}",
        details.as_ref().map(|r| (
            r.bond_information.node_id,
            &r.bond_information.node.identity_key
        ))
    );
    log::trace!("<<< {:?}", details);
    Ok(details)
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
    mix_id: NodeId,
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
) -> Result<LegacyNodeDescription, BackendError> {
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
pub async fn get_nym_node_description(
    host: &str,
    port: u16,
) -> Result<NodeDescription, BackendError> {
    Ok(
        nym_node_requests::api::Client::builder::<_, ErrorResponse>(format!(
            "http://{host}:{port}"
        ))?
        .with_timeout(Duration::from_millis(1000))
        .with_user_agent(format!("nym-wallet/{}", env!("CARGO_PKG_VERSION")))
        .build::<ErrorResponse>()?
        .get_description()
        .await?,
    )
}

#[tauri::command]
pub async fn get_mixnode_uptime(
    mix_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<u8, BackendError> {
    log::info!(">>> Get mixnode uptime");

    let guard = state.read().await;
    let client = guard.current_client()?;
    let uptime = client.nym_api.get_mixnode_avg_uptime(mix_id).await?;

    log::info!(">>> Uptime response: {}", uptime.avg_uptime);
    Ok(uptime.avg_uptime)
}

#[tauri::command]
pub async fn migrate_legacy_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.write().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    info!(">>> migrate to NymNode, fee = {fee:?}");
    let client = guard.current_client()?;

    let res = client.nyxd.migrate_legacy_mixnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn migrate_legacy_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.write().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    info!(">>> migrate to NymNode, fee = {fee:?}");
    let client = guard.current_client()?;

    let res = client.nyxd.migrate_legacy_gateway(None, fee).await?;

    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn update_nymnode_config(
    update: NodeConfigUpdate,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.write().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> update nym node config: update = {update:?}, fee {fee:?}",);
    let res = guard
        .current_client()?
        .nyxd
        .update_nymnode_config(update, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn get_nymnode_performance(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<f64>, BackendError> {
    log::trace!("  >>> Get node performance: node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nym_api
        .get_current_node_performance(node_id)
        .await?;
    log::trace!("  <<< {res:?}");

    Ok(res.performance)
}

#[tauri::command]
pub async fn get_nymnode_uptime(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<u8, BackendError> {
    log::info!(">>> Get legacy nymnode uptime");

    let performance = get_nymnode_performance(node_id, state).await?;

    // convert value in range 0.0 - 1.0 into 0-100
    Ok(performance
        .map(|p| (p * 100.).floor() as u8)
        .unwrap_or_default())
}

#[tauri::command]
pub async fn get_nymnode_stake_saturation(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<StakeSaturationResponse, BackendError> {
    log::trace!("  >>> Get node stake saturation: node_id = {node_id}");

    let res = nyxd_client!(state)
        .get_node_stake_saturation(node_id)
        .await?;
    log::trace!("  <<< {res:?}");

    Ok(res)
}
