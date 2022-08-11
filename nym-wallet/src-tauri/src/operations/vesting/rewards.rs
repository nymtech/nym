use crate::error::BackendError;
use crate::state::WalletState;
use mixnet_contract_common::IdentityKey;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn vesting_claim_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // log::info!(">>> Vesting account: claim operator reward");
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .execute_vesting_claim_operator_reward(None)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn vesting_compound_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // log::info!(">>> Vesting account: compound operator reward");
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .execute_vesting_compound_operator_reward(fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn vesting_claim_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // log::info!(
    //     ">>> Vesting account: claim delegator reward: identity_key = {}",
    //     mix_identity
    // );
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .execute_vesting_claim_delegator_reward(mix_identity, fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn vesting_compound_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // log::info!(
    //     ">>> Vesting account: compound delegator reward: identity_key = {}",
    //     mix_identity
    // );
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .execute_vesting_compound_delegator_reward(mix_identity, fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}
