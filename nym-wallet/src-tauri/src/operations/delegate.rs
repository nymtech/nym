use crate::coin::Coin;
use crate::format_err;
use crate::state::State;
use cosmwasm_std::Coin as CosmWasmCoin;
use mixnet_contract::{Addr, PagedReverseMixDelegationsResponse};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize)]
pub struct DelegationResult {
  source_address: String,
  target_address: String,
  amount: Option<Coin>,
}

#[tauri::command]
pub async fn delegate_to_mixnode(
  identity: &str,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match amount.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let client = r_state.client()?;
  match client.delegate_to_mixnode(identity, &bond).await {
    Ok(_result) => Ok(DelegationResult {
      source_address: client.address().to_string(),
      target_address: identity.to_string(),
      amount: Some(bond.into()),
    }),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.remove_mixnode_delegation(identity).await {
    Ok(_result) => Ok(DelegationResult {
      source_address: client.address().to_string(),
      target_address: identity.to_string(),
      amount: None,
    }),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn get_reverse_mix_delegations_paged(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<PagedReverseMixDelegationsResponse, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  client
    .get_reverse_mix_delegations_paged(Addr::unchecked(client.address().as_ref()), None, None)
    .await
    .map_err(|err| format_err!(err))
}
