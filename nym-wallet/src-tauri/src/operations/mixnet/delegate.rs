use crate::error::BackendError;
use crate::state::State;
use crate::vesting::delegate::{
    get_pending_vesting_delegation_events, vesting_undelegate_from_mixnode,
};
use crate::{api_client, nymd_client};
use cosmwasm_std::Coin as CosmWasmCoin;
use mixnet_contract_common::IdentityKey;
use nym_types::currency::{CurrencyDenom, MajorCurrencyAmount};
use nym_types::delegation::{
    from_contract_delegation_events, Delegation, DelegationEvent, DelegationRecord,
    DelegationWithEverything, DelegationsSummaryResponse,
};
use nym_types::transaction::TransactionExecuteResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::Fee;

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
    amount: MajorCurrencyAmount,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().denom();
    let delegation = amount.clone().into();
    log::info!(
        ">>> Delegate to mixnode: identity_key = {}, amount = {}, minor_amount = {}, fee = {:?}",
        identity,
        amount,
        delegation,
        fee,
    );
    let res = nymd_client!(state)
        .delegate_to_mixnode(identity, delegation, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
    identity: &str,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().denom();
    log::info!(
        ">>> Undelegate from mixnode: identity_key = {}, fee = {:?}",
        identity,
        fee
    );
    let res = nymd_client!(state)
        .remove_mixnode_delegation(identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn undelegate_all_from_mixnode(
    identity: &str,
    uses_vesting_contract_tokens: bool,
    fee: Option<Fee>,
    fee2: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<TransactionExecuteResult>, BackendError> {
    log::info!(
        ">>> Undelegate all from mixnode: identity_key = {}, uses_vesting_contract_tokens = {}, fee = {:?}",
        identity,
        uses_vesting_contract_tokens,
        fee
    );
    let mut res: Vec<TransactionExecuteResult> =
        vec![undelegate_from_mixnode(identity, fee, state.clone()).await?];

    if uses_vesting_contract_tokens {
        res.push(vesting_undelegate_from_mixnode(identity, fee2, state.clone()).await?);
    }

    Ok(res)
}

struct DelegationWithHistory {
    pub delegation: Delegation,
    pub amount_sum: MajorCurrencyAmount,
    pub history: Vec<DelegationRecord>,
    pub uses_vesting_contract_tokens: bool,
}

#[tauri::command]
pub async fn get_all_mix_delegations(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationWithEverything>, BackendError> {
    log::info!(">>> Get all mixnode delegations");

    // TODO: add endpoint to validator API to get a single mix node bond
    let mixnodes = api_client!(state).get_mixnodes().await?;

    let address = nymd_client!(state).address().to_string();

    let denom_minor = state.read().await.current_network().denom();
    let denom: CurrencyDenom = denom_minor.clone().try_into()?;

    log::info!("  >>> Get delegations");
    let delegations = nymd_client!(state)
        .get_delegator_delegations_paged(address.clone(), None, None) // get all delegations, ignoring paging
        .await?
        .delegations;
    log::info!("  <<< {} delegations", delegations.len());

    // first get pending events from the mixnet contract (operations made with unlocked tokens)
    let mut pending_events_for_account = get_pending_delegation_events(state.clone()).await?;

    // then get pending events from the vesting contract (operations made with locked tokens)
    let pending_vesting_events = get_pending_vesting_delegation_events(state.clone()).await?;
    for event in pending_vesting_events {
        pending_events_for_account.push(event);
    }

    log::info!(
        "  <<< {} pending delegation events for account",
        pending_events_for_account.len()
    );

    let mut map: HashMap<String, DelegationWithHistory> = HashMap::new();

    for pending_event in pending_events_for_account.clone() {
        if delegations
            .iter()
            .any(|d| d.node_identity == pending_event.node_identity)
        {
            let amount = pending_event
                .amount
                .unwrap_or_else(|| MajorCurrencyAmount::zero(&denom));
            let delegation = DelegationWithHistory {
                delegation: Delegation {
                    amount: amount.clone(),
                    node_identity: pending_event.node_identity,
                    proxy: pending_event.proxy,
                    owner: pending_event.address,
                    block_height: pending_event.block_height,
                },
                amount_sum: amount,
                uses_vesting_contract_tokens: false,
                history: vec![],
            };
            map.insert(delegation.delegation.node_identity.clone(), delegation);
        }
    }

    for d in delegations {
        // create record of delegation
        let delegated_on_iso_datetime = nymd_client!(state)
            .get_block_timestamp(Some(d.block_height as u32))
            .await?
            .to_rfc3339();
        let amount: MajorCurrencyAmount = d.amount.clone().into();
        let record = DelegationRecord {
            amount: amount.clone(),
            block_height: d.block_height,
            delegated_on_iso_datetime,
            uses_vesting_contract_tokens: d.proxy.is_some(),
        };

        let entry = map
            .entry(d.node_identity.clone())
            .or_insert(DelegationWithHistory {
                delegation: d.try_into()?,
                history: vec![],
                amount_sum: MajorCurrencyAmount::zero(&amount.denom),
                uses_vesting_contract_tokens: false,
            });

        entry.history.push(record);
        entry.amount_sum = entry.amount_sum.clone() + amount;
        entry.uses_vesting_contract_tokens =
            entry.uses_vesting_contract_tokens || entry.delegation.proxy.is_some();
    }

    let mut with_everything: Vec<DelegationWithEverything> = vec![];

    for item in map {
        let d = item.1.delegation;
        let history = item.1.history;
        let Delegation {
            owner,
            node_identity,
            amount,
            block_height,
            proxy,
        } = d;
        let uses_vesting_contract_tokens = item.1.uses_vesting_contract_tokens;

        log::trace!(
            "  --- Delegation: node_identity = {}, amount = {}",
            node_identity,
            amount
        );

        let mixnode = mixnodes
            .iter()
            .find(|m| m.mix_node.identity_key == node_identity);

        let pledge_amount: Option<MajorCurrencyAmount> =
            mixnode.and_then(|m| m.pledge_amount.clone().try_into().ok());

        let total_delegation: Option<MajorCurrencyAmount> =
            mixnode.and_then(|m| m.total_delegation.clone().try_into().ok());

        let profit_margin_percent: Option<u8> = mixnode.map(|m| m.mix_node.profit_margin_percent);

        log::trace!("  >>> Get accumulated rewards: address = {}", address);
        let accumulated_rewards = match nymd_client!(state)
            .get_delegator_rewards(address.clone(), node_identity.clone(), proxy.clone())
            .await
        {
            Ok(rewards) => {
                let reward = CosmWasmCoin {
                    denom: denom_minor.to_string(),
                    amount: rewards,
                };
                let amount = MajorCurrencyAmount::from(reward);
                log::trace!("  <<< rewards = {}, amount = {}", rewards, amount);
                Some(amount)
            }
            Err(_) => {
                log::trace!("  <<< no rewards waiting");
                None
            }
        };

        let pending_events =
            filter_pending_events(&node_identity, pending_events_for_account.clone());
        log::trace!(
            "  --- pending events for mixnode = {}",
            pending_events.len()
        );

        log::trace!(
            "  >>> Get stake saturation: node_identity = {}",
            node_identity
        );
        let stake_saturation = api_client!(state)
            .get_mixnode_stake_saturation(&node_identity)
            .await
            .ok()
            .map(|r| r.saturation);
        log::trace!("  <<< {:?}", stake_saturation);

        log::trace!(
            "  >>> Get average uptime percentage: node_identity = {}",
            node_identity
        );
        let avg_uptime_percent = api_client!(state)
            .get_mixnode_avg_uptime(&node_identity)
            .await
            .ok()
            .map(|r| r.avg_uptime);
        log::trace!("  <<< {:?}", avg_uptime_percent);

        log::trace!(
            "  >>> Convert delegated on block height to timestamp: block_height = {}",
            d.block_height
        );
        let timestamp = nymd_client!(state)
            .get_block_timestamp(Some(d.block_height as u32))
            .await?;
        let delegated_on_iso_datetime = timestamp.to_rfc3339();
        log::trace!(
            "  <<< timestamp = {:?}, delegated_on_iso_datetime = {:?}",
            timestamp,
            delegated_on_iso_datetime
        );

        with_everything.push(DelegationWithEverything {
            owner: owner.to_string(),
            node_identity: node_identity.to_string(),
            amount: item.1.amount_sum,
            block_height,
            uses_vesting_contract_tokens,
            delegated_on_iso_datetime,
            stake_saturation,
            accumulated_rewards,
            profit_margin_percent,
            pledge_amount,
            avg_uptime_percent,
            total_delegation,
            pending_events,
            history,
        })
    }
    log::trace!("<<< {:?}", with_everything);

    Ok(with_everything)
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
) -> Result<MajorCurrencyAmount, BackendError> {
    let denom_minor = state.read().await.current_network().denom();
    log::info!(
        ">>> Get delegator rewards: mix_identity = {}, proxy = {:?}",
        mix_identity,
        proxy
    );
    let res = nymd_client!(state)
        .get_delegator_rewards(address, mix_identity, proxy)
        .await?;
    let coin = CosmWasmCoin::new(res.u128(), denom_minor.as_ref());
    let amount = coin.into();
    log::info!(">>> res = {}, amount = {}", res, amount);
    Ok(amount)
}

#[tauri::command]
pub async fn get_delegation_summary(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationsSummaryResponse, BackendError> {
    log::info!(">>> Get delegation summary");

    let denom_minor = state.read().await.current_network().denom();
    let denom: CurrencyDenom = denom_minor.clone().try_into()?;

    let delegations = get_all_mix_delegations(state.clone()).await?;
    let mut total_delegations = MajorCurrencyAmount::zero(&denom);
    let mut total_rewards = MajorCurrencyAmount::zero(&denom);

    for d in delegations.clone() {
        total_delegations = total_delegations + d.amount;
        if let Some(rewards) = d.accumulated_rewards {
            total_rewards = total_rewards + rewards;
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
