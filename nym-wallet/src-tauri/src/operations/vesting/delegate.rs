use crate::error::BackendError;
use crate::state::WalletState;
use mixnet_contract_common::NodeId;
use nym_types::currency::DecCoin;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::{Fee, VestingSigningClient};

#[tauri::command]
pub async fn vesting_delegate_to_mixnode(
    identity: &str,
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    //   let guard = state.read().await;
    //   let delegation = guard.attempt_convert_to_base_coin(amount.clone())?;
    //   let fee_amount = guard.convert_tx_fee(fee.as_ref());
    //
    //   log::info!(
    //   ">>> Delegate to mixnode with locked tokens: identity_key = {}, amount_display = {}, amount_base = {}, fee = {:?}",
    //   identity,
    //   amount,
    //   delegation,
    //   fee
    // );
    //   let res = guard
    //       .current_client()?
    //       .nymd
    //       .vesting_delegate_to_mixnode(identity, delegation, fee)
    //       .await?;
    //   log::info!("<<< tx hash = {}", res.transaction_hash);
    //   log::trace!("<<< {:?}", res);
    //   Ok(TransactionExecuteResult::from_execute_result(
    //       res, fee_amount,
    //   )?)
}

#[tauri::command]
pub async fn vesting_undelegate_from_mixnode(
    mix_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // log::info!(
    //     ">>> Undelegate from mixnode delegated with locked tokens: identity_key = {}, fee = {:?}",
    //     identity,
    //     fee,
    // );
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .vesting_undelegate_from_mixnode(identity, fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}
