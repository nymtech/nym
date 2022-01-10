use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use cosmwasm_std::Timestamp;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::VestingQueryClient;

#[tauri::command]
pub async fn locked_coins(
  address: &str,
  block_time: Option<u64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .locked_coins(address, block_time.map(Timestamp::from_seconds))
      .await?
      .into(),
  )
}

#[tauri::command]
pub async fn spendable_coins(
  vesting_account_address: &str,
  block_time: Option<u64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .spendable_coins(
        vesting_account_address,
        block_time.map(Timestamp::from_seconds),
      )
      .await?
      .into(),
  )
}

#[tauri::command]
pub async fn vested_coins(
  vesting_account_address: &str,
  block_time: Option<u64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .vested_coins(
        vesting_account_address,
        block_time.map(Timestamp::from_seconds),
      )
      .await?
      .into(),
  )
}

#[tauri::command]
pub async fn vesting_coins(
  vesting_account_address: &str,
  block_time: Option<u64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .vesting_coins(
        vesting_account_address,
        block_time.map(Timestamp::from_seconds),
      )
      .await?
      .into(),
  )
}

#[tauri::command]
pub async fn vesting_start_time(
  vesting_account_address: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<u64, BackendError> {
  Ok(
    nymd_client!(state)
      .vesting_start_time(vesting_account_address)
      .await?
      .seconds(),
  )
}

#[tauri::command]
pub async fn vesting_end_time(
  vesting_account_address: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<u64, BackendError> {
  Ok(
    nymd_client!(state)
      .vesting_end_time(vesting_account_address)
      .await?
      .seconds(),
  )
}

#[tauri::command]
pub async fn original_vesting(
  vesting_account_address: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .original_vesting(vesting_account_address)
      .await?
      .into(),
  )
}

#[tauri::command]
pub async fn delegated_free(
  vesting_account_address: &str,
  block_time: Option<u64>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .delegated_free(
        vesting_account_address,
        block_time.map(Timestamp::from_seconds),
      )
      .await?
      .into(),
  )
}

#[tauri::command]
pub async fn delegated_vesting(
  block_time: Option<u64>,
  vesting_account_address: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  Ok(
    nymd_client!(state)
      .delegated_vesting(
        vesting_account_address,
        block_time.map(Timestamp::from_seconds),
      )
      .await?
      .into(),
  )
}
