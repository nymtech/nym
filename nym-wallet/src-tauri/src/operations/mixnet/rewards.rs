use crate::error::BackendError;
use crate::state::WalletState;
use crate::vesting::rewards::vesting_claim_delegator_reward;
use nym_mixnet_contract_common::{NodeId, RewardingParams};
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, MixnetSigningClient, NymContractsProvider, PagedMixnetQueryClient,
};
use nym_validator_client::nyxd::Fee;

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
        .nyxd
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
    node_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Withdraw delegator reward: node_id = {node_id}");
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    let res = guard
        .current_client()?
        .nyxd
        .withdraw_delegator_reward(node_id, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn claim_locked_and_unlocked_delegator_reward(
    node_id: NodeId,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<TransactionExecuteResult>, BackendError> {
    log::info!(">>> Claim delegator reward (locked and unlocked): node_id = {node_id}",);

    let guard = state.read().await;
    let client = guard.current_client()?;

    log::trace!(">>> Get delegations: node_id = {node_id}");
    let address = client.nyxd.address();
    let delegations = client.nyxd.get_all_delegator_delegations(&address).await?;
    log::trace!("<<< {} delegations", delegations.len());

    let vesting_contract = client
        .nyxd
        .vesting_contract_address()
        .expect("vesting contract address is not available")
        .to_string();
    let liquid_delegation = client
        .nyxd
        .get_delegation_details(node_id, &address, None)
        .await?;
    let vesting_delegation = client
        .nyxd
        .get_delegation_details(node_id, &address, Some(vesting_contract))
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
        res.push(claim_delegator_reward(node_id, fee.clone(), state.clone()).await?);
    }
    if did_delegate_with_vesting_contract {
        res.push(vesting_claim_delegator_reward(node_id, fee, state).await?);
    }
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_current_rewarding_parameters(
    state: tauri::State<'_, WalletState>,
) -> Result<RewardingParams, BackendError> {
    log::info!(">>> Get current rewarding params",);

    let guard = state.read().await;
    let client = guard.current_client()?;

    let reward_params = client.nyxd.get_rewarding_parameters().await?;

    Ok(reward_params)
}
