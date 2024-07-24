use crate::error::BackendError;
use crate::nyxd_client;
use crate::operations::helpers::{
    verify_gateway_bonding_sign_payload, verify_mixnode_bonding_sign_payload,
};
use crate::state::WalletState;
use crate::{Gateway, MixNode};
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::{GatewayConfigUpdate, MixNodeConfigUpdate};
use nym_types::currency::DecCoin;
use nym_types::mixnode::MixNodeCostParams;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::{contract_traits::VestingSigningClient, Fee};
use std::cmp::Ordering;

#[tauri::command]
pub async fn vesting_bond_gateway(
    gateway: Gateway,
    pledge: DecCoin,
    msg_signature: MessageSignature,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Bond gateway with locked tokens: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
        gateway.identity_key,
        pledge,
        pledge_base,
        fee,
    );

    let client = guard.current_client()?;
    // check the signature to make sure the user copied it correctly
    if let Err(err) =
        verify_gateway_bonding_sign_payload(client, &gateway, &pledge_base, true, &msg_signature)
            .await
    {
        log::warn!("failed to verify provided gateway bonding signature: {err}");
        return Err(err);
    }
    let res = guard
        .current_client()?
        .nyxd
        .vesting_bond_gateway(gateway, msg_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_unbond_gateway(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Unbond gateway bonded with locked tokens, fee = {:?}",
        fee
    );
    let res = nyxd_client!(state).vesting_unbond_gateway(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_bond_mixnode(
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    msg_signature: MessageSignature,
    pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let pledge_base = guard.attempt_convert_to_base_coin(pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let cost_params = cost_params.try_convert_to_mixnet_contract_cost_params(reg)?;

    log::info!(
      ">>> Bond mixnode with locked tokens: identity_key = {}, pledge_display = {}, pledge_base = {}, fee = {:?}",
      mixnode.identity_key,
      pledge,
      pledge_base,
      fee
    );

    let client = guard.current_client()?;
    // check the signature to make sure the user copied it correctly
    if let Err(err) = verify_mixnode_bonding_sign_payload(
        client,
        &mixnode,
        &cost_params,
        &pledge_base,
        true,
        &msg_signature,
    )
    .await
    {
        log::warn!("failed to verify provided mixnode bonding signature: {err}");
        return Err(err);
    }

    let res = guard
        .current_client()?
        .nyxd
        .vesting_bond_mixnode(mixnode, cost_params, msg_signature, pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_update_pledge(
    current_pledge: DecCoin,
    new_pledge: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let dec_delta = guard.calculate_coin_delta(&current_pledge, &new_pledge)?;
    let delta = guard.attempt_convert_to_base_coin(dec_delta.clone())?;
    log::info!(
        ">>> Pledge update, current pledge {}, new pledge {}",
        &current_pledge,
        &new_pledge,
    );

    let res = match new_pledge.amount.cmp(&current_pledge.amount) {
        Ordering::Greater => {
            log::info!(
                "Pledge increase with locked tokens, calculated additional pledge {}, fee = {:?}",
                dec_delta,
                fee,
            );
            guard
                .current_client()?
                .nyxd
                .vesting_pledge_more(delta, fee)
                .await?
        }
        Ordering::Less => {
            log::info!(
                "Pledge reduction with locked tokens, calculated reduction pledge {}, fee = {:?}",
                dec_delta,
                fee,
            );
            guard
                .current_client()?
                .nyxd
                .vesting_decrease_pledge(delta, fee)
                .await?
        }
        Ordering::Equal => return Err(BackendError::WalletPledgeUpdateNoOp),
    };

    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);

    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_pledge_more(
    fee: Option<Fee>,
    additional_pledge: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let additional_pledge_base = guard.attempt_convert_to_base_coin(additional_pledge.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Pledge more with locked tokens, additional_pledge_display = {}, additional_pledge_base = {}, fee = {:?}",
        additional_pledge,
        additional_pledge_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_pledge_more(additional_pledge_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_decrease_pledge(
    fee: Option<Fee>,
    decrease_by: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let decrease_by_base = guard.attempt_convert_to_base_coin(decrease_by.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Decrease pledge with locked tokens, pledge_decrease_display = {}, pledge_decrease_base = {}, fee = {:?}",
        decrease_by,
        decrease_by_base,
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_decrease_pledge(decrease_by_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_unbond_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Unbond mixnode bonded with locked tokens, fee = {:?}",
        fee
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_unbond_mixnode(fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn withdraw_vested_coins(
    amount: DecCoin,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let amount_base = guard.attempt_convert_to_base_coin(amount.clone())?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());

    log::info!(
        ">>> Withdraw vested liquid coins: amount_base = {}, amount_base = {}, fee = {:?}",
        amount,
        amount_base,
        fee
    );
    let res = guard
        .current_client()?
        .nyxd
        .withdraw_vested_coins(amount_base, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_update_mixnode_cost_params(
    new_costs: MixNodeCostParams,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let cost_params = new_costs.try_convert_to_mixnet_contract_cost_params(reg)?;

    log::info!(
        ">>> Update mixnode cost params with locked tokens: parameters = {}, fee = {:?}",
        cost_params.to_inline_json(),
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_update_mixnode_cost_params(cost_params, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_update_mixnode_config(
    update: MixNodeConfigUpdate,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Update mixnode config with locked tokens: update = {}, fee = {:?}",
        update.to_inline_json(),
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_update_mixnode_config(update, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn vesting_update_gateway_config(
    update: GatewayConfigUpdate,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Update gateway config with locked tokens: update = {}, fee = {:?}",
        update.to_inline_json(),
        fee,
    );
    let res = guard
        .current_client()?
        .nyxd
        .vesting_update_gateway_config(update, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}
