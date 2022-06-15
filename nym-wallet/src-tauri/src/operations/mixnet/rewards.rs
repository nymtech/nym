use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use mixnet_contract_common::IdentityKey;
use nym_types::transaction::TransactionExecuteResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn claim_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Claim operator reward");
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_claim_operator_reward(fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn compound_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Compound operator reward");
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_compound_operator_reward(fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn claim_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(
        ">>> Claim delegator reward: identity_key = {}",
        mix_identity
    );
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_claim_delegator_reward(mix_identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn compound_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(
        ">>> Compound delegator reward: identity_key = {}",
        mix_identity
    );
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_compound_delegator_reward(mix_identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}
