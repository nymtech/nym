use std::sync::Arc;

use tokio::sync::RwLock;

use nym_types::currency::MajorCurrencyAmount;
use nym_types::delegation::{from_contract_delegation_events, DelegationEvent};
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::{Fee, VestingSigningClient};

use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;

#[tauri::command]
pub async fn get_pending_vesting_delegation_events(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DelegationEvent>, BackendError> {
    log::info!(">>> Get pending delegations from vesting contract");

    let guard = state.read().await;
    let client = &guard.current_client()?.nymd;
    let vesting_contract = client.vesting_contract_address()?;

    let events = client
        .get_pending_delegation_events(
            client.address().to_string(),
            Some(vesting_contract.to_string()),
        )
        .await?;

    log::info!("<<< {} events", events.len());
    log::trace!("<<< {:?}", events);

    match from_contract_delegation_events(events) {
        Ok(res) => Ok(res),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn vesting_delegate_to_mixnode(
    identity: &str,
    amount: MajorCurrencyAmount,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().denom();
    let delegation = amount.clone().into();
    log::info!(
    ">>> Delegate to mixnode with locked tokens: identity_key = {}, amount = {}, minor_amount = {}, fee = {:?}",
    identity,
    amount,
    delegation,
    fee
  );
    let res = nymd_client!(state)
        .vesting_delegate_to_mixnode(identity, delegation, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn vesting_undelegate_from_mixnode(
    identity: &str,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().denom();
    log::info!(
        ">>> Undelegate from mixnode delegated with locked tokens: identity_key = {}, fee = {:?}",
        identity,
        fee,
    );
    let res = nymd_client!(state)
        .vesting_undelegate_from_mixnode(identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}
