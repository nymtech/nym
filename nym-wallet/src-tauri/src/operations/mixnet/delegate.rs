use crate::error::BackendError;
use crate::state::State;
use crate::vesting::delegate::get_pending_vesting_delegation_events;
use crate::{api_client, nymd_client};
use cosmwasm_std::Coin as CosmWasmCoin;
use mixnet_contract_common::IdentityKey;
use nym_types::currency::{CurrencyDenom, DecCoin};
use nym_types::delegation::{
    from_contract_delegation_events, Delegation, DelegationEvent, DelegationRecord,
    DelegationWithEverything, DelegationsSummaryResponse,
};
use nym_types::transaction::TransactionExecuteResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{Coin, Fee};

#[tauri::command]
pub async fn get_pending_delegation_events(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    log::info!(">>> Get pending delegation events");
    let events = nymd_client!(state)
        .get_pending_delegation_events(nymd_client!(state).address().to_string(), None)
        .await?;
    log::info!("<<< {} pending delegation events", events.len());
    log::trace!("<<< pending delegation events = {:?}", events);

    match from_contract_delegation_events(events) {
        Ok(res) => Ok(res),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delegate_to_mixnode(
    identity: &str,
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let delegation_base = guard.attempt_convert_to_base_coin(amount.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Delegate to mixnode: identity_key = {}, display_amount = {}, base_amount = {}, fee = {:?}",
        identity,
        amount,
        delegation_base,
        fee,
    );
    let res = nymd_client!(state)
        .delegate_to_mixnode(identity, delegation_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
    identity: &str,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Undelegate from mixnode: identity_key = {}, fee = {:?}",
        identity,
        fee
    );
    let res = guard
        .current_client()?
        .nymd
        .remove_mixnode_delegation(identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

struct DelegationWithHistory {
    pub delegation: Delegation,
    pub amount_sum: DecCoin,
    pub history: Vec<DelegationRecord>,
}

#[tauri::command]
pub async fn get_all_mix_delegations(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationWithEverything>, BackendError> {
    todo!("deal with this later :)");

    log::info!(">>> Get all mixnode delegations");
    //
    // // TODO: add endpoint to validator API to get a single mix node bond
    // let mixnodes = api_client!(state).get_mixnodes().await?;
    //
    // let address = nymd_client!(state).address().to_string();
    //
    // let denom_minor = state.read().await.current_network().denom();
    // let denom: CurrencyDenom = denom_minor.clone().try_into()?;
    //
    // log::info!("  >>> Get delegations");
    // let delegations = nymd_client!(state)
    //     .get_delegator_delegations_paged(address.clone(), None, None) // get all delegations, ignoring paging
    //     .await?
    //     .delegations;
    // log::info!("  <<< {} delegations", delegations.len());
    //
    // // first get pending events from the mixnet contract (operations made with unlocked tokens)
    // let mut pending_events_for_account = get_pending_delegation_events(state.clone()).await?;
    //
    // // then get pending events from the vesting contract (operations made with locked tokens)
    // let pending_vesting_events = get_pending_vesting_delegation_events(state.clone()).await?;
    // for event in pending_vesting_events {
    //     pending_events_for_account.push(event);
    // }
    //
    // log::info!(
    //     "  <<< {} pending delegation events for account",
    //     pending_events_for_account.len()
    // );
    //
    // let mut map: HashMap<String, DelegationWithHistory> = HashMap::new();
    //
    // for pending_event in pending_events_for_account.clone() {
    //     if delegations
    //         .iter()
    //         .any(|d| d.node_identity == pending_event.node_identity)
    //     {
    //         let amount = pending_event
    //             .amount
    //             .unwrap_or_else(|| DecCoin::zero(&denom));
    //         let delegation = DelegationWithHistory {
    //             delegation: Delegation {
    //                 amount: amount.clone(),
    //                 node_identity: pending_event.node_identity,
    //                 proxy: None,
    //                 owner: pending_event.address,
    //                 block_height: pending_event.block_height,
    //             },
    //             amount_sum: amount,
    //             history: vec![],
    //         };
    //         map.insert(delegation.delegation.node_identity.clone(), delegation);
    //     }
    // }
    //
    // for d in delegations {
    //     // create record of delegation
    //     let delegated_on_iso_datetime = nymd_client!(state)
    //         .get_block_timestamp(Some(d.block_height as u32))
    //         .await?
    //         .to_rfc3339();
    //     let amount: DecCoin = d.amount.clone().into();
    //     let record = DelegationRecord {
    //         amount: amount.clone(),
    //         block_height: d.block_height,
    //         delegated_on_iso_datetime,
    //     };
    //
    //     let entry = map
    //         .entry(d.node_identity.clone())
    //         .or_insert(DelegationWithHistory {
    //             delegation: d.try_into()?,
    //             history: vec![],
    //             amount_sum: DecCoin::zero(&amount.denom),
    //         });
    //
    //     entry.history.push(record);
    //     entry.amount_sum = entry.amount_sum.clone() + amount;
    // }
    //
    // let mut with_everything: Vec<DelegationWithEverything> = vec![];
    //
    // for item in map {
    //     let d = item.1.delegation;
    //     let history = item.1.history;
    //     let Delegation {
    //         owner,
    //         node_identity,
    //         amount,
    //         block_height,
    //         proxy,
    //     } = d;
    //
    //     log::trace!(
    //         "  --- Delegation: node_identity = {}, amount = {}",
    //         node_identity,
    //         amount
    //     );
    //
    //     let mixnode = mixnodes
    //         .iter()
    //         .find(|m| m.mix_node.identity_key == node_identity);
    //
    //     let pledge_amount: Option<DecCoin> =
    //         mixnode.and_then(|m| m.pledge_amount.clone().try_into().ok());
    //
    //     let total_delegation: Option<DecCoin> =
    //         mixnode.and_then(|m| m.total_delegation.clone().try_into().ok());
    //
    //     let profit_margin_percent: Option<u8> = mixnode.map(|m| m.mix_node.profit_margin_percent);
    //
    //     log::trace!("  >>> Get accumulated rewards: address = {}", address);
    //     let accumulated_rewards = match nymd_client!(state)
    //         .get_delegator_rewards(address.clone(), node_identity.clone(), proxy.clone())
    //         .await
    //     {
    //         Ok(rewards) => {
    //             let reward = CosmWasmCoin {
    //                 denom: denom_minor.to_string(),
    //                 amount: rewards,
    //             };
    //             let amount = DecCoin::from(reward);
    //             log::trace!("  <<< rewards = {}, amount = {}", rewards, amount);
    //             Some(amount)
    //         }
    //         Err(_) => {
    //             log::trace!("  <<< no rewards waiting");
    //             None
    //         }
    //     };
    //
    //     let pending_events =
    //         filter_pending_events(&node_identity, pending_events_for_account.clone());
    //     log::trace!(
    //         "  --- pending events for mixnode = {}",
    //         pending_events.len()
    //     );
    //
    //     log::trace!(
    //         "  >>> Get stake saturation: node_identity = {}",
    //         node_identity
    //     );
    //     let stake_saturation = api_client!(state)
    //         .get_mixnode_stake_saturation(&node_identity)
    //         .await
    //         .ok()
    //         .map(|r| r.saturation);
    //     log::trace!("  <<< {:?}", stake_saturation);
    //
    //     log::trace!(
    //         "  >>> Get average uptime percentage: node_identity = {}",
    //         node_identity
    //     );
    //     let avg_uptime_percent = api_client!(state)
    //         .get_mixnode_avg_uptime(&node_identity)
    //         .await
    //         .ok()
    //         .map(|r| r.avg_uptime);
    //     log::trace!("  <<< {:?}", avg_uptime_percent);
    //
    //     log::trace!(
    //         "  >>> Convert delegated on block height to timestamp: block_height = {}",
    //         d.block_height
    //     );
    //     let timestamp = nymd_client!(state)
    //         .get_block_timestamp(Some(d.block_height as u32))
    //         .await?;
    //     let delegated_on_iso_datetime = timestamp.to_rfc3339();
    //     log::trace!(
    //         "  <<< timestamp = {:?}, delegated_on_iso_datetime = {:?}",
    //         timestamp,
    //         delegated_on_iso_datetime
    //     );
    //
    //     with_everything.push(DelegationWithEverything {
    //         owner: owner.to_string(),
    //         node_identity: node_identity.to_string(),
    //         amount: item.1.amount_sum,
    //         block_height,
    //         proxy: proxy.clone(),
    //         delegated_on_iso_datetime,
    //         stake_saturation,
    //         accumulated_rewards,
    //         profit_margin_percent,
    //         pledge_amount,
    //         avg_uptime_percent,
    //         total_delegation,
    //         pending_events,
    //         history,
    //     })
    // }
    // log::trace!("<<< {:?}", with_everything);
    //
    // Ok(with_everything)
}

fn filter_pending_events(
    node_identity: &str,
    pending_events: Vec<DelegationEvent>,
) -> Vec<DelegationEvent> {
    pending_events
        .iter()
        .filter(|e| e.node_identity == node_identity)
        .cloned()
        .collect::<Vec<DelegationEvent>>()
}

#[tauri::command]
pub async fn get_delegator_rewards(
    address: String,
    mix_identity: IdentityKey,
    proxy: Option<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DecCoin, BackendError> {
    log::info!(
        ">>> Get delegator rewards: mix_identity = {}, proxy = {:?}",
        mix_identity,
        proxy
    );
    let guard = state.read().await;
    let network = guard.current_network();
    let denom = network.base_mix_denom();
    let reward_amount = guard
        .current_client()?
        .nymd
        .get_delegator_rewards(address, mix_identity, proxy)
        .await?;
    let base_coin = Coin::new(reward_amount.u128(), denom);
    let display_coin: DecCoin = guard.attempt_convert_to_display_dec_coin(base_coin.clone())?;

    log::info!(
        "<<< rewards_base = {}, rewards_display = {}",
        base_coin,
        display_coin
    );
    Ok(display_coin)
}

#[tauri::command]
pub async fn get_delegation_summary(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationsSummaryResponse, BackendError> {
    todo!("also deal with later : )");
    log::info!(">>> Get delegation summary");

    // let denom_minor = state.read().await.current_network().denom();
    // let denom: CurrencyDenom = denom_minor.clone().try_into()?;
    //
    // let delegations = get_all_mix_delegations(state.clone()).await?;
    // let mut total_delegations = DecCoin::zero(&denom);
    // let mut total_rewards = DecCoin::zero(&denom);
    //
    // for d in delegations.clone() {
    //     total_delegations = total_delegations + d.amount;
    //     if let Some(rewards) = d.accumulated_rewards {
    //         total_rewards = total_rewards + rewards;
    //     }
    // }
    // log::info!(
    //     "<<< {} delegations, total_delegations = {}, total_rewards = {}",
    //     delegations.len(),
    //     total_delegations,
    //     total_rewards
    // );
    // log::trace!("<<< {:?}", delegations);
    //
    // Ok(DelegationsSummaryResponse {
    //     delegations,
    //     total_delegations,
    //     total_rewards,
    // })
}

#[tauri::command]
pub async fn get_all_pending_delegation_events(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    log::info!(">>> Get all pending delegation events");

    // get pending events from mixnet and vesting contract
    let mut pending_events_for_account = get_pending_delegation_events(state.clone()).await?;
    let pending_vesting_events = get_pending_vesting_delegation_events(state.clone()).await?;

    // combine them
    for event in pending_vesting_events {
        pending_events_for_account.push(event);
    }

    Ok(pending_events_for_account)
}
