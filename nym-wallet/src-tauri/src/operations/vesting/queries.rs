// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::nyxd_client;
use crate::state::WalletState;
use cosmwasm_std::Timestamp;
use nym_types::currency::DecCoin;
use nym_types::vesting::VestingAccountInfo;
use nym_types::vesting::{OriginalVestingResponse, PledgeData};
use nym_validator_client::nyxd::contract_traits::VestingQueryClient;
use nym_vesting_contract_common::Period;

#[tauri::command]
pub(crate) async fn locked_coins(
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query locked coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .locked_coins(
            client.nyxd.address().as_ref(),
            block_time.map(Timestamp::from_seconds),
        )
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< locked coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn spendable_coins(
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query spendable coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .spendable_coins(
            client.nyxd.address().as_ref(),
            block_time.map(Timestamp::from_seconds),
        )
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< spendable coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn spendable_vested_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query spendable vested coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_spendable_vested_coins(client.nyxd.address().as_ref())
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< spendable vested coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn spendable_reward_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query spendable reward coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_spendable_reward_coins(client.nyxd.address().as_ref())
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< spendable reward coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn vested_coins(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query vested coins");
    let guard = state.read().await;

    let res = guard
        .current_client()?
        .nyxd
        .vested_coins(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< vested coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn vesting_coins(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query vesting coins");
    let guard = state.read().await;

    let res = guard
        .current_client()?
        .nyxd
        .vesting_coins(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< vesting coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn vesting_start_time(
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<u64, BackendError> {
    log::debug!(">>> Query vesting start time");
    let res = nyxd_client!(state)
        .vesting_start_time(vesting_account_address)
        .await?
        .seconds();
    log::debug!("<<< vesting start time = {res}");
    Ok(res)
}

#[tauri::command]
pub(crate) async fn vesting_end_time(
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<u64, BackendError> {
    log::debug!(">>> Query vesting end time");
    let res = nyxd_client!(state)
        .vesting_end_time(vesting_account_address)
        .await?
        .seconds();
    log::debug!("<<< vesting end time = {res}");
    Ok(res)
}

#[tauri::command]
pub(crate) async fn original_vesting(
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<OriginalVestingResponse, BackendError> {
    log::debug!(">>> Query original vesting");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;

    let res = guard
        .current_client()?
        .nyxd
        .original_vesting(vesting_account_address)
        .await?;

    let res = OriginalVestingResponse::from_vesting_contract(res, reg)?;
    log::debug!("<<< {res:?}");
    Ok(res)
}

#[tauri::command]
pub(crate) async fn get_historical_vesting_staking_reward(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query historical vesting staking reward coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_historical_vesting_staking_reward(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< historical vesting staking reward coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn get_spendable_vested_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query spendable vested coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_spendable_vested_coins(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< spendable vested coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn get_spendable_reward_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query spendable reward coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_spendable_reward_coins(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< spendable reward coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn get_delegated_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query delegated coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_delegated_coins(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< delegated coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn get_pledged_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query pledged coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_pledged_coins(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< pledged coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn get_staked_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query staked coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_staked_coins(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< staked coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn get_withdrawn_coins(
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::debug!(">>> Query withdrawn coins");
    let guard = state.read().await;
    let client = guard.current_client()?;

    let res = client
        .nyxd
        .get_withdrawn_coins(client.nyxd.address().as_ref())
        .await?;
    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::debug!("<<< withdrawn coins = {display}");
    Ok(display)
}

#[tauri::command]
pub(crate) async fn delegated_free(
    _vesting_account_address: &str,
    _block_time: Option<u64>,
    _state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::warn!(">>> Query delegated free -> THIS QUERY HAS BEEN REMOVED FROM THE CONTRACT");
    Err(BackendError::RemovedCommand {
        name: "vesting::queries::delegated_free".to_string(),
        alternative: "vesting::queries::get_delegated_coins".to_string(),
    })
}

/// Returns the total amount of delegated tokens that have vested
#[tauri::command]
pub(crate) async fn delegated_vesting(
    _block_time: Option<u64>,
    _vesting_account_address: &str,
    _state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::warn!(">>> Query delegated vesting -> THIS QUERY HAS BEEN REMOVED FROM THE CONTRACT");
    Err(BackendError::RemovedCommand {
        name: "vesting::queries::delegated_vesting".to_string(),
        alternative: "vesting::queries::get_delegated_coins".to_string(),
    })
}

#[tauri::command]
pub(crate) async fn vesting_get_mixnode_pledge(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<PledgeData>, BackendError> {
    log::debug!(">>> Query vesting get mixnode pledge");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;

    let res = guard
        .current_client()?
        .nyxd
        .get_mixnode_pledge(address)
        .await?
        .map(|pledge| PledgeData::from_vesting_contract(pledge, reg))
        .transpose()?;

    log::debug!("<<< {res:?}");
    Ok(res)
}

#[tauri::command]
pub(crate) async fn vesting_get_gateway_pledge(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<PledgeData>, BackendError> {
    log::debug!(">>> Query vesting get gateway pledge");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;

    let res = guard
        .current_client()?
        .nyxd
        .get_gateway_pledge(address)
        .await?
        .map(|pledge| PledgeData::from_vesting_contract(pledge, reg))
        .transpose()?;

    log::debug!("<<< {res:?}");
    Ok(res)
}

#[tauri::command]
pub(crate) async fn get_current_vesting_period(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<Period, BackendError> {
    log::debug!(">>> Query current vesting period");
    let res = nyxd_client!(state)
        .get_current_vesting_period(address)
        .await?;
    log::debug!("<<< {res:?}");
    Ok(res)
}

#[tauri::command]
pub(crate) async fn get_account_info(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<VestingAccountInfo, BackendError> {
    log::debug!(">>> Query account info");
    let guard = state.read().await;
    let res = guard.registered_coins()?;

    let vesting_account = guard.current_client()?.nyxd.get_account(address).await?;
    let res = VestingAccountInfo::from_vesting_contract(vesting_account, res)?;

    log::debug!("<<< {res:?}");
    Ok(res)
}
