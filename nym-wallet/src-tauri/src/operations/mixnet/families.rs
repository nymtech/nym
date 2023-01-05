use crate::error::BackendError;
use crate::state::WalletState;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::traits::MixnetSigningClient;
use validator_client::nymd::Fee;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn create_family(
    signature: String,
    label: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nymd
        .create_family(signature, label, fee)
        .await?;
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn join_family(
    signature: String,
    family_head: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nymd
        .join_family(signature, family_head, fee)
        .await?;
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn leave_family(
    signature: String,
    family_head: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nymd
        .leave_family(signature, family_head, fee)
        .await?;
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn kick_family_member(
    signature: String,
    member: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nymd
        .kick_family_member(signature, member, fee)
        .await?;
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}
