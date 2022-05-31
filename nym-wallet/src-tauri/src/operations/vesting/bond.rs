use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::{Gateway, MixNode};
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{Fee, VestingSigningClient};

#[tauri::command]
pub async fn vesting_bond_gateway(
    gateway: Gateway,
    pledge: Coin,
    owner_signature: String,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    let pledge = pledge.into_backend_coin(state.read().await.current_network().denom())?;
    nymd_client!(state)
        .vesting_bond_gateway(gateway, &owner_signature, pledge, fee)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state).vesting_unbond_gateway(fee).await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state).vesting_unbond_mixnode(fee).await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: Coin,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    let pledge = pledge.into_backend_coin(state.read().await.current_network().denom())?;
    nymd_client!(state)
        .vesting_bond_mixnode(mixnode, &owner_signature, pledge, fee)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn withdraw_vested_coins(
    amount: Coin,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    let amount = amount.into_backend_coin(state.read().await.current_network().denom())?;
    nymd_client!(state)
        .withdraw_vested_coins(amount, fee)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn vesting_update_mixnode(
    profit_margin_percent: u8,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    nymd_client!(state)
        .vesting_update_mixnode_config(profit_margin_percent, fee)
        .await?;
    Ok(())
}
