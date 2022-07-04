use crate::error::BackendError;
use crate::state::WalletState;
use crate::vesting::delegate::{
    get_pending_vesting_delegation_events, vesting_undelegate_from_mixnode,
};
use crate::{api_client, nymd_client};
use mixnet_contract_common::IdentityKey;
use nym_types::currency::DecCoin;
use nym_types::delegation::{
    Delegation, DelegationEvent, DelegationRecord, DelegationWithEverything,
    DelegationsSummaryResponse,
};
use nym_types::transaction::TransactionExecuteResult;
use std::collections::HashMap;
use validator_client::nymd::{Coin, Fee};

#[tauri::command]
pub async fn get_pending_delegation_events(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    log::info!(">>> Get pending delegation events");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = guard.current_client()?;

    let events = client
        .nymd
        .get_pending_delegation_events(client.nymd.address().to_string(), None)
        .await?;
    log::info!("<<< {} pending delegation events", events.len());
    log::trace!("<<< pending delegation events = {:?}", events);

    Ok(events
        .into_iter()
        .map(|event| DelegationEvent::from_mixnet_contract(event, reg))
        .collect::<Result<_, _>>()?)
}

#[tauri::command]
pub async fn delegate_to_mixnode(
    identity: &str,
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
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
    state: tauri::State<'_, WalletState>,
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

#[tauri::command]
pub async fn undelegate_all_from_mixnode(
    identity: &str,
    uses_vesting_contract_tokens: bool,
    fee: Option<Fee>,
    fee2: Option<Fee>,
    state: tauri::State<'_, WalletState>,
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

    // TODO: add endpoint to validator API to get a single mix node bond
    let mixnodes = client.validator_api.get_mixnodes().await?;

    let address = client.nymd.address();
    let network = guard.current_network();
    let display_mix_denom = network.display_mix_denom();
    let base_mix_denom = network.base_mix_denom();

    log::info!("  >>> Get delegations");
    let delegations = client.get_all_delegator_delegations(address).await?;
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

    for pending_event in &pending_events_for_account {
        if delegations
            .iter()
            .any(|d| d.node_identity == pending_event.node_identity)
        {
            let amount = pending_event
                .amount
                .clone()
                .unwrap_or_else(|| DecCoin::zero(display_mix_denom));
            let delegation = DelegationWithHistory {
                delegation: Delegation {
                    amount: amount.clone(),
                    node_identity: pending_event.node_identity.clone(),
                    proxy: pending_event.proxy.clone(), // TODO: ask @MS about delegations via vesting contract => surely we'd have proxy there?
                    owner: pending_event.address.clone(),
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
        let delegated_on_iso_datetime = client
            .nymd
            .get_block_timestamp(Some(d.block_height as u32))
            .await?
            .to_rfc3339();
        let amount = guard.attempt_convert_to_display_dec_coin(d.amount.clone().into())?;

        let record = DelegationRecord {
            amount: amount.clone(),
            block_height: d.block_height,
            delegated_on_iso_datetime,
            uses_vesting_contract_tokens: d.proxy.is_some(),
        };

        let entry = map
            .entry(d.node_identity.clone())
            .or_insert(DelegationWithHistory {
                delegation: Delegation::from_mixnet_contract(d, reg)?,
                history: vec![],
                amount_sum: DecCoin::zero(display_mix_denom),
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

        let pledge_amount = mixnode
            .map(|m| guard.attempt_convert_to_display_dec_coin(m.original_pledge.clone().into()))
            .transpose()?;

        let total_delegation = mixnode
            .map(|m| guard.attempt_convert_to_display_dec_coin(m.total_delegation.clone().into()))
            .transpose()?;

        let profit_margin_percent: Option<u8> = mixnode.map(|m| m.mix_node.profit_margin_percent);

        log::trace!("  >>> Get accumulated rewards: address = {}", address);
        let accumulated_rewards = match client
            .nymd
            .get_delegator_rewards(address.to_string(), node_identity.clone(), proxy.clone())
            .await
        {
            Ok(rewards) => {
                let reward = Coin::new(rewards.u128(), base_mix_denom);
                let amount = guard.attempt_convert_to_display_dec_coin(reward)?;
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
    state: tauri::State<'_, WalletState>,
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
        if let Some(rewards) = &d.accumulated_rewards {
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

#[tauri::command]
pub async fn get_all_pending_delegation_events(
    state: tauri::State<'_, WalletState>,
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
