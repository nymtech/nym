use std::sync::Arc;

use cosmwasm_std::Timestamp;
use tokio::sync::RwLock;

use nym_types::currency::MajorCurrencyAmount;
use nym_types::vesting::VestingAccountInfo;
use nym_types::vesting::{OriginalVestingResponse, PledgeData};
use validator_client::nymd::VestingQueryClient;
use vesting_contract_common::Period;

use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;

#[tauri::command]
pub async fn locked_coins(
    block_time: Option<u64>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Query locked coins");
    let res = nymd_client!(state)
        .locked_coins(
            nymd_client!(state).address().as_ref(),
            block_time.map(Timestamp::from_seconds),
        )
        .await?
        .into();
    log::info!("<<< locked coins = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn spendable_coins(
    block_time: Option<u64>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Query spendable coins");
    let res = nymd_client!(state)
        .spendable_coins(
            nymd_client!(state).address().as_ref(),
            block_time.map(Timestamp::from_seconds),
        )
        .await?
        .into();
    log::info!("<<< spendable coins = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vested_coins(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Query vested coins");
    let res = nymd_client!(state)
        .vested_coins(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?
        .into();
    log::info!("<<< vested coins = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_coins(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Query vesting coins");
    let res = nymd_client!(state)
        .vesting_coins(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?
        .into();
    log::info!("<<< vesting coins = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_start_time(
    vesting_account_address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<u64, BackendError> {
    log::info!(">>> Query vesting start time");
    let res = nymd_client!(state)
        .vesting_start_time(vesting_account_address)
        .await?
        .seconds();
    log::info!("<<< vesting start time = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_end_time(
    vesting_account_address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<u64, BackendError> {
    log::info!(">>> Query vesting end time");
    let res = nymd_client!(state)
        .vesting_end_time(vesting_account_address)
        .await?
        .seconds();
    log::info!("<<< vesting end time = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn original_vesting(
    vesting_account_address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<OriginalVestingResponse, BackendError> {
    log::info!(">>> Query original vesting");
    let res = nymd_client!(state)
        .original_vesting(vesting_account_address)
        .await?
        .try_into()?;
    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn delegated_free(
    vesting_account_address: &str,
    block_time: Option<u64>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Query delegated free");
    let res = nymd_client!(state)
        .delegated_free(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?
        .into();
    log::info!("<<< delegated free = {}", res);
    Ok(res)
}

/// Returns the total amount of delegated tokens that have vested
#[tauri::command]
pub async fn delegated_vesting(
    block_time: Option<u64>,
    vesting_account_address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Query delegated vesting");
    let res = nymd_client!(state)
        .delegated_vesting(
            vesting_account_address,
            block_time.map(Timestamp::from_seconds),
        )
        .await?
        .into();
    log::info!("<<< delegated_vesting = {}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_get_mixnode_pledge(
    address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Option<PledgeData>, BackendError> {
    log::info!(">>> Query vesting get mixnode pledge");
    let res = nymd_client!(state)
        .get_mixnode_pledge(address)
        .await?
        .and_then(PledgeData::and_then);
    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn vesting_get_gateway_pledge(
    address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Option<PledgeData>, BackendError> {
    log::info!(">>> Query vesting get gateway pledge");
    let res = nymd_client!(state)
        .get_gateway_pledge(address)
        .await?
        .and_then(PledgeData::and_then);
    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_current_vesting_period(
    address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Period, BackendError> {
    log::info!(">>> Query current vesting period");
    let res = nymd_client!(state)
        .get_current_vesting_period(address)
        .await?;
    log::info!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_account_info(
    address: &str,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<VestingAccountInfo, BackendError> {
    log::info!(">>> Query account info");
    let res = nymd_client!(state).get_account(address).await?.try_into()?;
    log::info!("<<< {:?}", res);
    Ok(res)
}
