// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::nyxd_client;
use crate::state::WalletState;
use cosmwasm_std::Timestamp;
use nym_types::currency::DecCoin;
use nym_types::vesting::VestingAccountInfo;
use nym_types::vesting::{OriginalVestingResponse, PledgeData};
use validator_client::nyxd::VestingQueryClient;
use vesting_contract_common::Period;

#[tauri::command]
pub async fn locked_coins(
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Query locked coins");
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
    log::info!("<<< locked coins = {}", display);
    Ok(display)
}

#[tauri::command]
pub async fn spendable_coins(
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Query spendable coins");
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
    log::info!("<<< spendable coins = {}", display);
    Ok(display)
}

#[tauri::command]
pub async fn vested_coins(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Query vested coins");
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
    log::info!("<<< vested coins = {}", display);
    Ok(display)
}

#[tauri::command]
pub async fn vesting_coins(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Query vesting coins");
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
    log::info!("<<< vesting coins = {}", display);
    Ok(display)
}

#[tauri::command]
pub async fn vesting_start_time(
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<u64, BackendError> {
    log::info!(">>> Query vesting start time");
    let res = nyxd_client!(state)
        .vesting_start_time(vesting_account_address)
        .await?
        .seconds();
    log::info!("<<< vesting start time = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_end_time(
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<u64, BackendError> {
    log::info!(">>> Query vesting end time");
    let res = nyxd_client!(state)
        .vesting_end_time(vesting_account_address)
        .await?
        .seconds();
    log::info!("<<< vesting end time = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn original_vesting(
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<OriginalVestingResponse, BackendError> {
    log::info!(">>> Query original vesting");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;

    let res = guard
        .current_client()?
        .nyxd
        .original_vesting(vesting_account_address)
        .await?;

    let res = OriginalVestingResponse::from_vesting_contract(res, reg)?;
    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn delegated_free(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Query delegated free");
    let guard = state.read().await;

    let res = guard
        .current_client()?
        .nyxd
        .delegated_free(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::info!("<<< delegated free = {}", display);
    Ok(display)
}

/// Returns the total amount of delegated tokens that have vested
#[tauri::command]
pub async fn delegated_vesting(
    block_time: Option<u64>,
    vesting_account_address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Query delegated vesting");
    let guard = state.read().await;

    let res = guard
        .current_client()?
        .nyxd
        .delegated_vesting(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?;

    let display = guard.attempt_convert_to_display_dec_coin(res)?;
    log::info!("<<< delegated_vesting = {}", display);
    Ok(display)
}

#[tauri::command]
pub async fn vesting_get_mixnode_pledge(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<PledgeData>, BackendError> {
    log::info!(">>> Query vesting get mixnode pledge");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;

    let res = guard
        .current_client()?
        .nyxd
        .get_mixnode_pledge(address)
        .await?
        .map(|pledge| PledgeData::from_vesting_contract(pledge, reg))
        .transpose()?;

    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_get_gateway_pledge(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<PledgeData>, BackendError> {
    log::info!(">>> Query vesting get gateway pledge");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;

    let res = guard
        .current_client()?
        .nyxd
        .get_gateway_pledge(address)
        .await?
        .map(|pledge| PledgeData::from_vesting_contract(pledge, reg))
        .transpose()?;

    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_current_vesting_period(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<Period, BackendError> {
    log::info!(">>> Query current vesting period");
    let res = nyxd_client!(state)
        .get_current_vesting_period(address)
        .await?;
    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_account_info(
    address: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<VestingAccountInfo, BackendError> {
    log::info!(">>> Query account info");
    let guard = state.read().await;
    let res = guard.registered_coins()?;

    let vesting_account = guard.current_client()?.nyxd.get_account(address).await?;
    let res = VestingAccountInfo::from_vesting_contract(vesting_account, res)?;

    log::info!("<<< {:?}", res);
    Ok(res)
}
