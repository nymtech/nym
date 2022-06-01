use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::utils::DelegationEvent;
use crate::utils::DelegationResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{Fee, VestingSigningClient};

#[tauri::command]
pub async fn get_pending_vesting_delegation_events(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    let guard = state.read().await;
    let client = &guard.current_client()?.nymd;
    let vesting_contract = client.vesting_contract_address()?;

    Ok(client
        .get_pending_delegation_events(
            client.address().to_string(),
            Some(vesting_contract.to_string()),
        )
        .await?
        .into_iter()
        .map(|delegation_event| delegation_event.into())
        .collect::<Vec<DelegationEvent>>())
}

#[tauri::command]
pub async fn vesting_delegate_to_mixnode(
    identity: &str,
    amount: Coin,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
    let delegation = amount.into_backend_coin(state.read().await.current_network().denom())?;
    nymd_client!(state)
        .vesting_delegate_to_mixnode(identity, delegation.clone(), fee)
        .await?;
    Ok(DelegationResult::new(
        nymd_client!(state).address().as_ref(),
        identity,
        Some(delegation.into()),
    ))
}

#[tauri::command]
pub async fn vesting_undelegate_from_mixnode(
    identity: &str,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, BackendError> {
    nymd_client!(state)
        .vesting_undelegate_from_mixnode(identity, fee)
        .await?;
    Ok(DelegationResult::new(
        nymd_client!(state).address().as_ref(),
        identity,
        None,
    ))
}
