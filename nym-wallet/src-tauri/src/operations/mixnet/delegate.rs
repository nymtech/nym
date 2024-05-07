// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use crate::vesting::delegate::vesting_undelegate_from_mixnode;
use nym_mixnet_contract_common::mixnode::StakeSaturationResponse;
use nym_mixnet_contract_common::MixId;
use nym_types::currency::DecCoin;
use nym_types::delegation::{Delegation, DelegationWithEverything, DelegationsSummaryResponse};
use nym_types::deprecated::{
    convert_to_delegation_events, DelegationEvent, WrappedDelegationEvent,
};
use nym_types::mixnode::MixNodeCostParams;
use nym_types::pending_events::PendingEpochEvent;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, MixnetSigningClient, NymContractsProvider, PagedMixnetQueryClient,
};
use nym_validator_client::nyxd::Fee;
use tap::TapFallible;

#[tauri::command]
pub async fn get_pending_delegation_events(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<WrappedDelegationEvent>, BackendError> {
    log::info!(">>> [DEPRECATED] Get all pending delegation events");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = guard.current_client()?;

    let events = client.nyxd.get_all_pending_epoch_events().await?;
    let converted = events
        .into_iter()
        .map(|e| PendingEpochEvent::try_from_mixnet_contract(e, reg))
        .collect::<Result<Vec<_>, _>>()?;

    let delegation_events = convert_to_delegation_events(converted);

    // we only care about events concerning THIS client
    let mut client_specific_events = Vec::new();
    for delegation_event in delegation_events {
        if delegation_event.address_matches(client.nyxd.address().as_ref()) {
            let node_identity = client
                .nyxd
                .get_mixnode_details(delegation_event.mix_id)
                .await?
                .mixnode_details
                .map(|d| d.bond_information.mix_node.identity_key)
                .unwrap_or_default();

            client_specific_events
                .push(WrappedDelegationEvent::new(delegation_event, node_identity));
        }
    }

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
    mix_id: MixId,
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
        .nyxd
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
    mix_id: MixId,
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
        .nyxd
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
    mix_id: MixId,
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

#[tauri::command]
pub async fn get_all_mix_delegations(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<DelegationWithEverything>, BackendError> {
    log::info!(">>> Get all mixnode delegations");

    let guard = state.read().await;
    let client = guard.current_client()?;
    let reg = guard.registered_coins()?;

    let address = client.nyxd.address();
    let network = guard.current_network();
    let base_mix_denom = network.base_mix_denom().to_string();
    let vesting_contract = client
        .nyxd
        .vesting_contract_address()
        .expect("vesting contract address is not available");

    log::info!("  >>> Get delegations");
    let delegations = client
        .nyxd
        .get_all_delegator_delegations(&address)
        .await
        .tap_err(|err| {
            log::error!("  <<< Failed to get delegations. Error: {}", err);
        })?;
    log::info!("  <<< {} delegations", delegations.len());

    let pending_events_for_account =
        get_pending_delegation_events(state.clone())
            .await
            .tap_err(|err| {
                log::error!("  <<< Failed to get pending delegations. Error: {}", err);
            })?;

    log::info!(
        "  <<< {} pending delegation events for account",
        pending_events_for_account.len()
    );

    let mut with_everything: Vec<DelegationWithEverything> = Vec::with_capacity(delegations.len());

    for delegation in delegations {
        let mut error_strings: Vec<String> = vec![];

        let d = Delegation::from_mixnet_contract(delegation.clone(), reg).tap_err(|err| {
            log::error!(
                "  <<< Failed to get delegation for mix id {} from contract. Error: {}",
                delegation.mix_id,
                err
            );
        })?;

        let uses_vesting_contract_tokens = d
            .proxy
            .as_ref()
            .map(|p| p.as_str() == vesting_contract.as_ref())
            .unwrap_or_default();

        log::trace!(
            "  --- Delegation: mix_id = {}, amount = {}",
            d.mix_id,
            d.amount
        );

        let mixnode = client
            .nyxd
            .get_mixnode_details(d.mix_id)
            .await
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to get mixnode details for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })?
            .mixnode_details;

        let accumulated_by_operator = mixnode
            .as_ref()
            .map(|m| {
                guard.display_coin_from_base_decimal(&base_mix_denom, m.rewarding_details.operator)
            })
            .transpose()
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to get operator rewards as a display coin for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })
            .unwrap_or_default();

        let accumulated_by_delegates = mixnode
            .as_ref()
            .map(|m| {
                guard.display_coin_from_base_decimal(&base_mix_denom, m.rewarding_details.delegates)
            })
            .transpose()
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to get delegator rewards as a display coin for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })
            .unwrap_or_default();

        let cost_params = mixnode
            .as_ref()
            .map(|m| {
                MixNodeCostParams::from_mixnet_contract_mixnode_cost_params(
                    m.rewarding_details.cost_params.clone(),
                    reg,
                )
            })
            .transpose()
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to mixnode cost params for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })
            .unwrap_or_default();

        log::trace!("  >>> Get accumulated rewards: address = {}", address);
        let pending_reward = client
            .nyxd
            .get_pending_delegator_reward(&address, d.mix_id, d.proxy.clone())
            .await
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to get accumulated rewards for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })
            .unwrap_or_default();

        let accumulated_rewards = match &pending_reward.amount_earned {
            Some(reward) => {
                let amount = guard
                    .attempt_convert_to_display_dec_coin(reward.clone().into())
                    .tap_err(|err| {
                        let str_err = format!("Failed to get convert reward to a display coin for mix_id = {}. Error: {}", d.mix_id, err);
                        log::error!("  <<< {}", str_err);
                        error_strings.push(str_err);
                    })
                    .ok();
                log::trace!(
                    "  <<< rewards = {:?}, amount = {:?}",
                    pending_reward,
                    amount
                );
                amount
            }
            None => {
                log::trace!("  <<< no rewards waiting");
                None
            }
        };

        log::trace!("  >>> Get stake saturation: mix_id = {}", d.mix_id);
        let stake_saturation = client
            .nyxd
            .get_mixnode_stake_saturation(d.mix_id)
            .await
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to get stake saturation for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })
            .unwrap_or(StakeSaturationResponse {
                mix_id: d.mix_id,
                uncapped_saturation: None,
                current_saturation: None,
            });
        log::trace!("  <<< {:?}", stake_saturation);

        log::trace!(
            "  >>> Get average uptime percentage: mix_iid = {}",
            d.mix_id
        );
        let avg_uptime_percent = client
            .nym_api
            .get_mixnode_avg_uptime(d.mix_id)
            .await
            .tap_err(|err| {
                let str_err = format!(
                    "Failed to get average uptime percentage for mix_id = {}. Error: {}",
                    d.mix_id, err
                );
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            })
            .ok()
            .map(|r| r.avg_uptime);
        log::trace!("  <<< {:?}", avg_uptime_percent);

        log::trace!(
            "  >>> Convert delegated on block height to timestamp: block_height = {}",
            d.height
        );
        let timestamp = client
            .nyxd
            .get_block_timestamp(Some(d.height as u32))
            .await
            .tap_err(|err| {
                let str_err = format!("Failed to get block timestamp for height = {} for delegation to mix_id = {}. Error: {}", d.height, d.mix_id, err);
                log::error!("  <<< {}", str_err);
                error_strings.push(str_err);
            }).ok();
        let delegated_on_iso_datetime = timestamp.map(|ts| ts.to_rfc3339());
        log::trace!(
            "  <<< timestamp = {:?}, delegated_on_iso_datetime = {:?}",
            timestamp,
            delegated_on_iso_datetime
        );

        let pending_events = filter_pending_events(d.mix_id, &pending_events_for_account);
        log::trace!(
            "  --- pending events for mixnode = {}",
            pending_events.len()
        );

        let mixnode_is_unbonding = mixnode.as_ref().map(|m| m.is_unbonding());
        log::trace!(
            "  >>> mixnode with mix_id: {} is unbonding: {:?}",
            d.mix_id,
            mixnode_is_unbonding
        );

        with_everything.push(DelegationWithEverything {
            owner: d.owner,
            mix_id: d.mix_id,
            node_identity: mixnode
                .map(|m| m.bond_information.mix_node.identity_key)
                .unwrap_or_default(),
            amount: d.amount,
            block_height: d.height,
            uses_vesting_contract_tokens,
            delegated_on_iso_datetime,
            stake_saturation: stake_saturation.uncapped_saturation,
            accumulated_by_operator,
            avg_uptime_percent,
            accumulated_by_delegates,
            cost_params,
            unclaimed_rewards: accumulated_rewards,
            pending_events,
            mixnode_is_unbonding,
            errors: if error_strings.is_empty() {
                None
            } else {
                Some(error_strings.join("\n"))
            },
        })
    }
    log::trace!("<<< {:?}", with_everything);

    Ok(with_everything)
}

fn filter_pending_events(
    mix_id: MixId,
    pending_events: &[WrappedDelegationEvent],
) -> Vec<DelegationEvent> {
    pending_events
        .iter()
        .filter(|e| e.event.mix_id == mix_id)
        .cloned()
        .map(|e| e.event)
        .collect()
}

#[tauri::command]
pub async fn get_pending_delegator_rewards(
    address: String,
    mix_id: MixId,
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
        .nyxd
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
