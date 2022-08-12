use crate::error::BackendError;
use crate::state::WalletState;
use crate::vesting::rewards::vesting_claim_delegator_reward;
use mixnet_contract_common::NodeId;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn claim_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    // // TODO: handle operator bonding with vesting contract
    log::info!(">>> Withdraw operator reward");
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nymd
        .withdraw_operator_reward(fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn claim_delegator_reward(
    mix_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Withdraw delegator reward: mix_id = {}", mix_id);
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nymd
        .withdraw_delegator_reward(mix_id, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn claim_locked_and_unlocked_delegator_reward(
    mix_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<TransactionExecuteResult>, BackendError> {
    log::info!(
        ">>> Claim delegator reward (locked and unlocked): mix_id = {}",
        mix_id
    );

    let guard = state.read().await;
    let client = guard.current_client()?;

    log::trace!(">>> Get delegations: mix_id = {}", mix_id);
    let address = client.nymd.address();
    let delegations = client.get_all_delegator_delegations(address).await?;
    log::trace!("<<< {} delegations", delegations.len());

    let vesting_contract = client.nymd.vesting_contract_address().to_string();
    let liquid_delegation = client.get_delegation_details(mix_id, address, None).await?;
    let vesting_delegation = client
        .get_delegation_details(mix_id, address, Some(vesting_contract))
        .await?;

    drop(guard);

    // decide which contracts to use, could be neither
    let did_delegate_with_mixnet_contract = liquid_delegation.delegation.is_some();
    let did_delegate_with_vesting_contract = vesting_delegation.delegation.is_some();

    log::trace!(
        "<<< Delegations done with: mixnet contract = {}, vesting contract = {}",
        did_delegate_with_mixnet_contract,
        did_delegate_with_vesting_contract
    );

    let mut res: Vec<TransactionExecuteResult> = vec![];
    if did_delegate_with_mixnet_contract {
        res.push(claim_delegator_reward(mix_id, fee.clone(), state.clone()).await?);
    }
    if did_delegate_with_vesting_contract {
        res.push(vesting_claim_delegator_reward(mix_id, fee, state).await?);
    }
    log::trace!("<<< {:?}", res);
    Ok(res)
}
