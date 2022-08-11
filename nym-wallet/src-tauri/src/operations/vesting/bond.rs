use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use crate::{Gateway, MixNode};

use nym_types::currency::DecCoin;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::{Fee, VestingSigningClient};

#[tauri::command]
pub async fn vesting_bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    owner_signature: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    //
    // log::info!(
    //     ">>> Bond gateway with locked tokens: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
    //     gateway.identity_key,
    //     pledge,
    //     pledge_base,
    //     fee,
    // );
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .vesting_bond_gateway(gateway, &owner_signature, pledge_base, fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn vesting_unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // log::info!(
    //     ">>> Unbond gateway bonded with locked tokens, fee = {:?}",
    //     fee
    // );
    // let res = nymd_client!(state).vesting_unbond_gateway(fee).await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn vesting_bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    //   let guard = state.read().await;
    //   let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    //   let fee_amount = guard.convert_tx_fee(fee.as_ref());
    //
    //   log::info!(
    //   ">>> Bond mixnode with locked tokens: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
    //   mixnode.identity_key,
    //   pledge,
    //   pledge_base,
    //   fee
    // );
    //   let res = guard
    //       .current_client()?
    //       .nymd
    //       .vesting_bond_mixnode(mixnode, &owner_signature, pledge_base, fee)
    //       .await?;
    //   log::info!("<<< tx hash = {}", res.transaction_hash);
    //   log::trace!("<<< {:?}", res);
    //   Ok(TransactionExecuteResult::from_execute_result(
    //       res, fee_amount,
    //   )?)
}

#[tauri::command]
pub async fn vesting_unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // log::info!(
    //     ">>> Unbond mixnode bonded with locked tokens, fee = {:?}",
    //     fee
    // );
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .vesting_unbond_mixnode(fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn withdraw_vested_coins(
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let amount_base = guard.attempt_convert_to_base_coin(amount.clone())?;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    //
    // log::info!(
    //     ">>> Withdraw vested liquid coins: amount_base = {}, amount_base = {}, fee = {:?}",
    //     amount,
    //     amount_base,
    //     fee
    // );
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .withdraw_vested_coins(amount_base, fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}

#[tauri::command]
pub async fn vesting_update_mixnode(
    profit_margin_percent: u8,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    todo!()
    // let guard = state.read().await;
    // let fee_amount = guard.convert_tx_fee(fee.as_ref());
    // log::info!(
    //     ">>> Update mixnode bonded with locked tokens: profit_margin_percent = {}, fee = {:?}",
    //     profit_margin_percent,
    //     fee,
    // );
    // let res = guard
    //     .current_client()?
    //     .nymd
    //     .vesting_update_mixnode_config(profit_margin_percent, fee)
    //     .await?;
    // log::info!("<<< tx hash = {}", res.transaction_hash);
    // log::trace!("<<< {:?}", res);
    // Ok(TransactionExecuteResult::from_execute_result(
    //     res, fee_amount,
    // )?)
}
