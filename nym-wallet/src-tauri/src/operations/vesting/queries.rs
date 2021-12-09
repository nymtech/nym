use crate::coin::Coin;
use crate::error::BackendError;
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
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
  let client = state.read().await.client()?;
  Ok(
    client
      .delegated_vesting(
        vesting_account_address,
        block_time.map(Timestamp::from_seconds),
      )
      .await?
      .into(),
  )
}
