use crate::coin::Coin;
use crate::error::BackendError;
use crate::state::State;
use crate::utils::DelegationResult;
use cosmwasm_std::Coin as CosmWasmCoin;
use mixnet_contract::PagedDelegatorDelegationsResponse;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn delegate_to_mixnode(
  identity: &str,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
  let client = state.read().await.client()?;
  let delegation: CosmWasmCoin = amount.try_into()?;
  client.delegate_to_mixnode(identity, &delegation).await?;
  Ok(DelegationResult::new(
    &client.address().to_string(),
    identity,
    Some(delegation.into()),
  ))
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
  let client = state.read().await.client()?;
  client.remove_mixnode_delegation(identity).await?;
  Ok(DelegationResult::new(
    &client.address().to_string(),
    identity,
    None,
  ))
}

#[tauri::command]
pub async fn get_reverse_mix_delegations_paged(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<PagedDelegatorDelegationsResponse, BackendError> {
  let client = state.read().await.client()?;
  Ok(
    client
      .get_delegator_delegations_paged(client.address().to_string(), None, None)
      .await?,
  )
}
