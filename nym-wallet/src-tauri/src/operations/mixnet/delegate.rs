use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::utils::DelegationResult;
use cosmwasm_std::Coin as CosmWasmCoin;
use mixnet_contract_common::PagedDelegatorDelegationsResponse;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn delegate_to_mixnode(
  identity: &str,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
  let delegation: CosmWasmCoin = amount.try_into()?;
  nymd_client!(state)
    .delegate_to_mixnode(identity, &delegation)
    .await?;
  Ok(DelegationResult::new(
    &nymd_client!(state).address().to_string(),
    identity,
    Some(delegation.into()),
  ))
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
  nymd_client!(state)
    .remove_mixnode_delegation(identity)
    .await?;
  Ok(DelegationResult::new(
    &nymd_client!(state).address().to_string(),
    identity,
    None,
  ))
}

#[tauri::command]
pub async fn get_reverse_mix_delegations_paged(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<PagedDelegatorDelegationsResponse, BackendError> {
  Ok(
    nymd_client!(state)
      .get_delegator_delegations_paged(nymd_client!(state).address().to_string(), None, None)
      .await?,
  )
}
