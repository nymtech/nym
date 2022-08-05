use crate::error::BackendError;
use crate::state::WalletState;
use crate::{Gateway, MixNode};
use nym_types::currency::DecCoin;
use nym_types::gateway::GatewayBond;
use nym_types::mixnode::MixNodeBond;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::{Coin, Fee};

#[tauri::command]
pub async fn bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    owner_signature: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Bond gateway: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
        gateway.identity_key,
        pledge,
        pledge_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .bond_gateway(gateway, owner_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> Unbond gateway, fee = {:?}", fee);
    let res = guard.current_client()?.nymd.unbond_gateway(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn bond_mixnode(
    mixnode: MixNode,
    owner_signature: String,
    pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Bond mixnode: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
        mixnode.identity_key,
        pledge,
        pledge_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .bond_mixnode(mixnode, owner_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> Unbond mixnode, fee = {:?}", fee);
    let res = guard.current_client()?.nymd.unbond_mixnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn update_mixnode(
    profit_margin_percent: u8,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Update mixnode: profit_margin_percent = {}, fee {:?}",
        profit_margin_percent,
        fee,
    );
    let res = guard
        .current_client()?
        .nymd
        .update_mixnode_config(profit_margin_percent, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn mixnode_bond_details(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<MixNodeBond>, BackendError> {
    log::info!(">>> Get mixnode bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let bond = client.nymd.owns_mixnode(client.nymd.address()).await?;
    let res = bond
        .map(|bond| {
            guard
                .registered_coins()
                .map(|reg| MixNodeBond::from_mixnet_contract_mixnode_bond(bond, reg))
        })
        .transpose()?
        .transpose()?;
    log::info!(
        "<<< identity_key = {:?}",
        res.as_ref().map(|r| r.mix_node.identity_key.to_string())
    );
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn gateway_bond_details(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<GatewayBond>, BackendError> {
    log::info!(">>> Get gateway bond details");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let bond = client.nymd.owns_gateway(client.nymd.address()).await?;
    let res = bond
        .map(|bond| {
            guard
                .registered_coins()
                .map(|reg| GatewayBond::from_mixnet_contract_gateway_bond(bond, reg))
        })
        .transpose()?
        .transpose()?;

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
    state: tauri::State<'_, WalletState>,
) -> Result<DecCoin, BackendError> {
    log::info!(">>> Get operator rewards for {}", address);
    let guard = state.read().await;
    let network = guard.current_network();
    let denom = network.base_mix_denom();
    let reward_amount = guard
        .current_client()?
        .nymd
        .get_operator_rewards(address)
        .await?;
    let base_coin = Coin::new(reward_amount.u128(), denom);
    let display_coin: DecCoin = guard.attempt_convert_to_display_dec_coin(base_coin.clone())?;
    log::info!(
        "<<< rewards_base = {}, rewards_display = {}",
        base_coin,
        display_coin
    );
    Ok(display_coin)
}

#[tauri::command]
pub async fn get_number_of_mixnode_delegators(identity: String, state: tauri::State<'_, WalletState>) -> Result<usize, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let paged_delegations = client.nymd.get_mix_delegations_paged(identity, None, None).await?;

    Ok(paged_delegations.delegations.len())
} 