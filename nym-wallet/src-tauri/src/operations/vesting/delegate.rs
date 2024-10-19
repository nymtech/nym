// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use nym_mixnet_contract_common::NodeId;
use nym_types::currency::DecCoin;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::{contract_traits::VestingSigningClient, Fee};

#[tauri::command]
pub async fn vesting_delegate_to_mixnode(
    mix_id: NodeId,
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let delegation = guard.attempt_convert_to_base_coin(amount.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
      ">>> Delegate to mixnode with locked tokens: mix_id = {}, amount_display = {}, amount_base = {}, fee = {:?}",
      mix_id,
      amount,
      delegation,
      fee
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_delegate_to_mixnode(mix_id, delegation, None, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_undelegate_from_mixnode(
    mix_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Undelegate from mixnode delegated with locked tokens: mix_id = {}, fee = {:?}",
        mix_id,
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_undelegate_from_mixnode(mix_id, None, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}
