use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use mixnet_contract_common::IdentityKey;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn claim_operator_reward(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_claim_operator_reward(None)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn compound_operator_reward(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_compound_operator_reward(None)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn claim_delegator_reward(
    mix_identity: IdentityKey,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_claim_delegator_reward(mix_identity, None)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn compound_delegator_reward(
    mix_identity: IdentityKey,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_compound_delegator_reward(mix_identity, None)
        .await?;
    Ok(())
}
