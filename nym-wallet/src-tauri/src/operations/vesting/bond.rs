use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::{Gateway, MixNode};

use nym_types::currency::MajorCurrencyAmount;
use nym_types::transaction::TransactionExecuteResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{Fee, VestingSigningClient};

#[tauri::command]
pub async fn vesting_bond_gateway(
    gateway: Gateway,
    pledge: MajorCurrencyAmount,
    owner_signature: String,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    let pledge_minor = pledge.clone().into();
    log::info!(
    ">>> Bond gateway with locked tokens: identity_key = {}, pledge = {}, pledge_minor = {}, fee = {:?}",
    gateway.identity_key,
    pledge,
    pledge_minor,
    fee,
  );
    let res = nymd_client!(state)
        .vesting_bond_gateway(gateway, &owner_signature, pledge_minor, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn vesting_unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    log::info!(
        ">>> Unbond gateway bonded with locked tokens, fee = {:?}",
        fee
    );
    let res = nymd_client!(state).vesting_unbond_gateway(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn vesting_bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: MajorCurrencyAmount,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    let pledge_minor = pledge.clone().into();
    log::info!(
    ">>> Bond mixnode with locked tokens: identity_key = {}, pledge = {}, pledge_minor = {}, fee = {:?}",
    mixnode.identity_key,
    pledge,
    pledge_minor,
    fee
  );
    let res = nymd_client!(state)
        .vesting_bond_mixnode(mixnode, &owner_signature, pledge_minor, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn vesting_unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    log::info!(
        ">>> Unbond mixnode bonded with locked tokens, fee = {:?}",
        fee
    );
    let res = nymd_client!(state).vesting_unbond_mixnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn withdraw_vested_coins(
    amount: MajorCurrencyAmount,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    let amount_minor = amount.clone().into();
    log::info!(
        ">>> Withdraw vested liquid coins: amount = {}, amount_minor = {}, fee = {:?}",
        amount,
        amount_minor,
        fee
    );
    let res = nymd_client!(state)
        .withdraw_vested_coins(amount_minor, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn vesting_update_mixnode(
    profit_margin_percent: u8,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    log::info!(
        ">>> Update mixnode bonded with locked tokens: profit_margin_percent = {}, fee = {:?}",
        profit_margin_percent,
        fee,
    );
    let res = nymd_client!(state)
        .vesting_update_mixnode_config(profit_margin_percent, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}
