use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::utils::DelegationEvent;
use crate::utils::DelegationResult;
use cosmwasm_std::Uint128;
use mixnet_contract_common::{IdentityKey, PagedDelegatorDelegationsResponse};
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn get_pending_delegation_events(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    Ok(nymd_client!(state)
        .get_pending_delegation_events(nymd_client!(state).address().to_string(), None)
        .await?
        .into_iter()
        .map(|delegation_event| delegation_event.into())
        .collect::<Vec<DelegationEvent>>())
}

#[tauri::command]
pub async fn delegate_to_mixnode(
    identity: &str,
    amount: Coin,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
    let delegation = amount.into_backend_coin(state.read().await.current_network().denom())?;
    nymd_client!(state)
        .delegate_to_mixnode(identity, delegation.clone(), fee)
        .await?;
    Ok(DelegationResult::new(
        nymd_client!(state).address().as_ref(),
        identity,
        Some(delegation.into()),
    ))
}

#[tauri::command]
pub async fn undelegate_from_mixnode(
    identity: &str,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
    nymd_client!(state)
        .remove_mixnode_delegation(identity, fee)
        .await?;
    Ok(DelegationResult::new(
        nymd_client!(state).address().as_ref(),
        identity,
        None,
    ))
}

#[tauri::command]
pub async fn get_reverse_mix_delegations_paged(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<PagedDelegatorDelegationsResponse, BackendError> {
    Ok(nymd_client!(state)
        .get_delegator_delegations_paged(nymd_client!(state).address().to_string(), None, None)
        .await?)
}

#[tauri::command]
pub async fn get_delegator_rewards(
    address: String,
    mix_identity: IdentityKey,
    proxy: Option<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Uint128, BackendError> {
    Ok(nymd_client!(state)
        .get_delegator_rewards(address, mix_identity, proxy)
        .await?)
}
