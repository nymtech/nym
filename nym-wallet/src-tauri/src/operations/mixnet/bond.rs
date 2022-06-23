use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::{Gateway, MixNode};
use nym_types::currency::MajorCurrencyAmount;
use nym_types::gateway::GatewayBond;
use nym_types::mixnode::MixNodeBond;
use nym_types::transaction::TransactionExecuteResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{CosmWasmCoin, Fee};

#[tauri::command]
pub async fn bond_gateway(
    gateway: Gateway,
    pledge: MajorCurrencyAmount,
    owner_signature: String,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    let pledge_minor = pledge.clone().into();
    log::info!(
        ">>> Bond gateway: identity_key = {}, pledge = {}, pledge_minor = {}, fee = {:?}",
        &gateway.identity_key,
        pledge,
        &pledge_minor,
        fee,
    );
    let res = nymd_client!(state)
        .bond_gateway(gateway, owner_signature, pledge_minor, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    log::info!(">>> Unbond gateway, fee = {:?}", fee);
    let res = nymd_client!(state).unbond_gateway(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: MajorCurrencyAmount,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    let pledge_minor = pledge.clone().into();
    log::info!(
        ">>> Bond mixnode: identity_key = {}, pledge = {}, pledge_minor = {}, fee = {:?}",
        mixnode.identity_key,
        pledge,
        pledge_minor,
        fee,
    );
    let res = nymd_client!(state)
        .bond_mixnode(mixnode, owner_signature, pledge_minor, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    log::info!(">>> Unbond mixnode, fee = {:?}", fee);
    let res = nymd_client!(state).unbond_mixnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn update_mixnode(
    profit_margin_percent: u8,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    log::info!(
        ">>> Update mixnode: profit_margin_percent = {}, fee {:?}",
        profit_margin_percent,
        fee,
    );
    let res = nymd_client!(state)
        .update_mixnode_config(profit_margin_percent, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn mixnode_bond_details(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Option<MixNodeBond>, BackendError> {
    log::info!(">>> Get mixnode bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let bond = client.nymd.owns_mixnode(client.nymd.address()).await?;
    let res = MixNodeBond::from_mixnet_contract_mixnode_bond(bond)?;
    log::info!(
        "<<< identity_key = {:?}",
        res.as_ref().map(|r| r.mix_node.identity_key.to_string())
    );
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn gateway_bond_details(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Option<GatewayBond>, BackendError> {
    log::info!(">>> Get gateway bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let bond = client.nymd.owns_gateway(client.nymd.address()).await?;
    let res = GatewayBond::from_mixnet_contract_gateway_bond(bond)?;
    log::info!(
        "<<< identity_key = {:?}",
        res.as_ref().map(|r| r.gateway.identity_key.to_string())
    );
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_operator_rewards(
    address: String,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
    log::info!(">>> Get operator rewards for {}", address);
    let denom = state.read().await.current_network().base_mix_denom();
    let rewards_as_minor = nymd_client!(state).get_operator_rewards(address).await?;
    let coin = CosmWasmCoin::new(rewards_as_minor.u128(), denom.as_ref());
    let amount: MajorCurrencyAmount = coin.into();
    log::info!(
        "<<< rewards_as_minor = {}, amount = {}",
        rewards_as_minor,
        amount
    );
    Ok(amount)
}
