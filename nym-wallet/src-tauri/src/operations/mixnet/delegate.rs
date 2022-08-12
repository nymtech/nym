// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use crate::vesting::delegate::vesting_undelegate_from_mixnode;
use mixnet_contract_common::NodeId;
use nym_types::currency::DecCoin;
use nym_types::delegation::{
    Delegation, DelegationRecord, DelegationWithEverything, DelegationsSummaryResponse,
};
use nym_types::deprecated::{convert_to_delegation_events, DelegationEvent};
use nym_types::mixnode::MixNodeCostParams;
use nym_types::pending_events::PendingEpochEvent;
use nym_types::transaction::TransactionExecuteResult;
use std::collections::HashMap;
use validator_client::nymd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn get_pending_delegation_events(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    log::info!(">>> [DEPRECATED] Get all pending delegation events");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = guard.current_client()?;

    let events = client.get_all_nymd_pending_epoch_events().await?;
    let converted = events
        .into_iter()
        .map(|e| PendingEpochEvent::try_from_mixnet_contract(e, reg))
        .collect::<Result<Vec<_>, _>>()?;

    let delegation_events = convert_to_delegation_events(converted);

    // we only care about events concerning THIS client
    let client_specific_events = delegation_events
        .into_iter()
        .filter(|e| e.address_matches(client.nymd.address().as_ref()))
        .collect::<Vec<_>>();

    log::info!(
        "<<< {} pending delegation events",
        client_specific_events.len()
    );
    log::trace!(
        "<<< pending delegation events = {:?}",
        client_specific_events
    );

    Ok(client_specific_events)
}

#[tauri::command]
pub async fn delegate_to_mixnode(
    mix_id: NodeId,
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let delegation_base = guard.attempt_convert_to_base_coin(amount.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Delegate to mixnode: mix_id = {}, display_amount = {}, base_amount = {}, fee = {:?}",
        mix_id,
        amount,
        delegation_base,
        fee,
    );
    let res = client
        .nymd
        .delegate_to_mixnode(mix_id, delegation_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
    mix_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Undelegate from mixnode: mix_id = {}, fee = {:?}",
        mix_id,
        fee
    );
    let res = guard
        .current_client()?
        .nymd
        .undelegate_from_mixnode(mix_id, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn undelegate_all_from_mixnode(
    mix_id: NodeId,
    uses_vesting_contract_tokens: bool,
    fee_liquid: Option<Fee>,
    fee_vesting: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<TransactionExecuteResult>, BackendError> {
    log::info!(
        ">>> Undelegate all from mixnode: mix_id = {}, uses_vesting_contract_tokens = {}, fee_liquid = {:?}, fee_vesting = {:?}",
        mix_id,
        uses_vesting_contract_tokens,
        fee_liquid,
        fee_vesting,
    );
    let mut res: Vec<TransactionExecuteResult> =
        vec![undelegate_from_mixnode(mix_id, fee_liquid, state.clone()).await?];

    if uses_vesting_contract_tokens {
        res.push(vesting_undelegate_from_mixnode(mix_id, fee_vesting, state).await?);
    }

    Ok(res)
}

struct DelegationWithHistory {
    pub delegation: Delegation,
    pub amount_sum: DecCoin,
    pub history: Vec<DelegationRecord>,
    pub uses_vesting_contract_tokens: bool,
}

#[tauri::command]
pub async fn get_all_mix_delegations(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<DelegationWithEverything>, BackendError> {
    log::info!(">>> Get all mixnode delegations");

    let guard = state.read().await;
    let client = guard.current_client()?;
    let reg = guard.registered_coins()?;

    let address = client.nymd.address();
    let network = guard.current_network();
    let display_mix_denom = network.display_mix_denom().to_string();
    let base_mix_denom = network.base_mix_denom().to_string();
    let vesting_contract = client.nymd.vesting_contract_address();

    log::info!("  >>> Get delegations");
    let delegations = client.get_all_delegator_delegations(address).await?;
    log::info!("  <<< {} delegations", delegations.len());

    // first get pending events from the mixnet contract (operations made with unlocked tokens)
    let pending_events_for_account = get_pending_delegation_events(state.clone()).await?;

    log::info!(
        "  <<< {} pending delegation events for account",
        pending_events_for_account.len()
    );

    let mut map: HashMap<NodeId, DelegationWithHistory> = HashMap::new();

    for d in delegations {
        if let Some(pending_event) = pending_events_for_account
            .iter()
            .find(|e| e.mix_id == d.node_id)
        {
            let amount = pending_event
                .amount
                .clone()
                .unwrap_or_else(|| DecCoin::zero(&display_mix_denom));
            let delegation = DelegationWithHistory {
                delegation: Delegation {
                    amount: amount.clone(),
                    proxy: pending_event.proxy.clone(),
                    owner: pending_event.address.clone(),
                    mix_id: pending_event.mix_id,
                    height: 0,
                },
                amount_sum: amount,
                uses_vesting_contract_tokens: false,
                history: vec![],
            };
            map.insert(delegation.delegation.mix_id, delegation);
        }

        // create record of delegation
        let delegated_on_iso_datetime = client
            .nymd
            .get_block_timestamp(Some(d.height as u32))
            .await?
            .to_rfc3339();
        let amount = guard.attempt_convert_to_display_dec_coin(d.amount.clone().into())?;

        let record = DelegationRecord {
            amount: amount.clone(),
            block_height: d.height,
            delegated_on_iso_datetime,
            uses_vesting_contract_tokens: d
                .proxy
                .as_ref()
                .map(|p| p.as_str() == vesting_contract.as_ref())
                .unwrap_or_default(),
        };

        let entry = map.entry(d.node_id).or_insert(DelegationWithHistory {
            delegation: Delegation::from_mixnet_contract(d, reg)?,
            history: vec![],
            amount_sum: DecCoin::zero(&display_mix_denom),
            uses_vesting_contract_tokens: false,
        });

        debug_assert_eq!(entry.amount_sum.denom, amount.denom);

        entry.history.push(record);
        entry.amount_sum.amount += amount.amount;
        entry.uses_vesting_contract_tokens =
            entry.uses_vesting_contract_tokens || entry.delegation.proxy.is_some();
    }

    let mut with_everything: Vec<DelegationWithEverything> = vec![];

    for item in map {
        let d = item.1.delegation;
        let history = item.1.history;
        let Delegation {
            owner,
            mix_id,
            amount,
            height,
            proxy,
        } = d;
        let uses_vesting_contract_tokens = item.1.uses_vesting_contract_tokens;

        log::trace!("  --- Delegation: mix_id = {}, amount = {}", mix_id, amount);

        let mixnode = client.get_mixnode_details(mix_id).await?.mixnode_details;

        let accumulated_by_operator = mixnode
            .as_ref()
            .map(|m| {
                guard.display_coin_from_base_decimal(&base_mix_denom, m.rewarding_details.operator)
            })
            .transpose()?;

        let accumulated_by_delegates = mixnode
            .as_ref()
            .map(|m| {
                guard.display_coin_from_base_decimal(&base_mix_denom, m.rewarding_details.delegates)
            })
            .transpose()?;

        let cost_params = mixnode
            .as_ref()
            .map(|m| {
                MixNodeCostParams::from_mixnet_contract_mixnode_cost_params(
                    m.rewarding_details.cost_params.clone(),
                    reg,
                )
            })
            .transpose()?;

        log::trace!("  >>> Get accumulated rewards: address = {}", address);
        let pending_reward = client
            .nymd
            .get_pending_delegator_reward(address, mix_id, proxy.clone())
            .await?;

        let accumulated_rewards = match &pending_reward.amount_earned {
            Some(reward) => {
                let amount = guard.attempt_convert_to_display_dec_coin(reward.clone().into())?;
                log::trace!("  <<< rewards = {:?}, amount = {}", pending_reward, amount);
                Some(amount)
            }
            None => {
                log::trace!("  <<< no rewards waiting");
                None
            }
        };

        log::trace!("  >>> Get stake saturation: mix_id = {}", mix_id);
        let stake_saturation = client.nymd.get_mixnode_stake_saturation(mix_id).await?;
        log::trace!("  <<< {:?}", stake_saturation);

        log::trace!("  >>> Get average uptime percentage: mix_iid = {}", mix_id);
        let avg_uptime_percent = client
            .validator_api
            .get_mixnode_avg_uptime(mix_id)
            .await
            .ok()
            .map(|r| r.avg_uptime);
        log::trace!("  <<< {:?}", avg_uptime_percent);

        log::trace!(
            "  >>> Convert delegated on block height to timestamp: block_height = {}",
            d.height
        );
        let timestamp = client
            .nymd
            .get_block_timestamp(Some(d.height as u32))
            .await?;
        let delegated_on_iso_datetime = timestamp.to_rfc3339();
        log::trace!(
            "  <<< timestamp = {:?}, delegated_on_iso_datetime = {:?}",
            timestamp,
            delegated_on_iso_datetime
        );

        with_everything.push(DelegationWithEverything {
            owner: owner.to_string(),
            mix_id,
            node_identity: mixnode
                .map(|m| m.bond_information.mix_node.identity_key)
                .unwrap_or_default(),
            amount: item.1.amount_sum,
            block_height: height,
            uses_vesting_contract_tokens,
            delegated_on_iso_datetime,
            stake_saturation: stake_saturation.uncapped_saturation,
            accumulated_by_operator,
            avg_uptime_percent,
            accumulated_by_delegates,
            history,
            cost_params,
            unclaimed_rewards: accumulated_rewards,
        })
    }
    log::trace!("<<< {:?}", with_everything);

    Ok(with_everything)
}

#[tauri::command]
pub async fn get_pending_delegator_rewards(
    address: String,
    mix_id: NodeId,
    proxy: Option<String>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(
        ">>> Get pending delegator rewards: mix_id = {}, proxy = {:?}",
        mix_id,
        proxy
    );
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nymd
        .get_pending_delegator_reward(&address.parse()?, mix_id, proxy)
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
pub async fn get_delegation_summary(
    state: tauri::State<'_, WalletState>,
) -> Result<DelegationsSummaryResponse, BackendError> {
    log::info!(">>> Get delegation summary");

    let guard = state.read().await;
    let network = guard.current_network();
    let display_mix_denom = network.display_mix_denom();

    let delegations = get_all_mix_delegations(state.clone()).await?;
    let mut total_delegations = DecCoin::zero(display_mix_denom);
    let mut total_rewards = DecCoin::zero(display_mix_denom);

    for d in &delegations {
        debug_assert_eq!(d.amount.denom, display_mix_denom);
        total_delegations.amount += d.amount.amount;
        if let Some(rewards) = &d.unclaimed_rewards {
            debug_assert_eq!(rewards.denom, display_mix_denom);
            total_rewards.amount += rewards.amount;
        }
    }
    log::info!(
        "<<< {} delegations, total_delegations = {}, total_rewards = {}",
        delegations.len(),
        total_delegations,
        total_rewards
    );
    log::trace!("<<< {:?}", delegations);

    Ok(DelegationsSummaryResponse {
        delegations,
        total_delegations,
        total_rewards,
    })
}
