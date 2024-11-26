// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use nym_mixnet_contract_common::NodeId;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::contract_traits::VestingSigningClient;
use nym_validator_client::nyxd::Fee;

#[tauri::command]
pub async fn vesting_claim_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Vesting account: claim operator reward");
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nyxd
        .vesting_withdraw_operator_reward(None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_claim_delegator_reward(
    mix_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(
        ">>> Vesting account: claim delegator reward: mix_id = {}",
        mix_id
    );
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nyxd
        .vesting_withdraw_delegator_reward(mix_id, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}
