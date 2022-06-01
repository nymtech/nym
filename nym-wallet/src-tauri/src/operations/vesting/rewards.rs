use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use mixnet_contract_common::IdentityKey;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn vesting_claim_operator_reward(
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_vesting_claim_operator_reward(None)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_compound_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_vesting_compound_operator_reward(fee)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_claim_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_vesting_claim_delegator_reward(mix_identity, fee)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_compound_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .execute_vesting_compound_delegator_reward(mix_identity, fee)
        .await?;
    Ok(())
}
