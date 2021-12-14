use crate::client;
use crate::coin::Coin;
use crate::error::BackendError;
use crate::state::State;
use crate::utils::DelegationResult;
use cosmwasm_std::Coin as CosmWasmCoin;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::VestingSigningClient;

#[tauri::command]
pub async fn vesting_delegate_to_mixnode(
  identity: &str,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
  let delegation: CosmWasmCoin = amount.try_into()?;
  client!(state)
    .vesting_delegate_to_mixnode(identity, &delegation)
    .await?;
  Ok(DelegationResult::new(
    &client!(state).address().to_string(),
    identity,
    Some(delegation.into()),
  ))
}

#[tauri::command]
pub async fn vesting_undelegate_from_mixnode(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
  client!(state)
    .vesting_undelegate_from_mixnode(identity)
    .await?;
  Ok(DelegationResult::new(
    &client!(state).address().to_string(),
    identity,
    None,
  ))
}
