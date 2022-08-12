use crate::error::BackendError;
use crate::state::WalletState;
use crate::vesting::rewards::vesting_claim_delegator_reward;
use mixnet_contract_common::NodeId;
use nym_types::transaction::TransactionExecuteResult;
use validator_client::nymd::traits::MixnetSigningClient;
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
    todo!()
    // log::info!(
    //     ">>> Claim delegator reward (locked and unlocked): identity_key = {}",
    //     mix_identity
    // );
    //
    // log::trace!(">>> Get delegations: identity_key = {}", mix_identity);
    // let address = nymd_client!(state).address().to_string();
    // let delegations = nymd_client!(state)
    //     .get_delegator_delegations_paged(address.clone(), None, None) // get all delegations, ignoring paging
    //     .await?
    //     .delegations;
    // log::trace!("<<< {} delegations", delegations.len());
    //
    // // decide which contracts to use, could be neither
    // let did_delegate_with_mixnet_contract = delegations
    //     .iter()
    //     .filter(|f| f.node_identity == mix_identity)
    //     .any(|f| f.proxy.is_none());
    // let did_delegate_with_vesting_contract = delegations
    //     .iter()
    //     .filter(|f| f.node_identity == mix_identity)
    //     .any(|f| f.proxy.is_some());
    //
    // log::trace!(
    //     "<<< Delegations done with: mixnet contract = {}, vesting contract = {}",
    //     did_delegate_with_mixnet_contract,
    //     did_delegate_with_vesting_contract
    // );
    //
    // let mut res: Vec<TransactionExecuteResult> = vec![];
    // if did_delegate_with_mixnet_contract {
    //     res.push(claim_delegator_reward(mix_identity.clone(), fee.clone(), state.clone()).await?);
    // }
    // if did_delegate_with_vesting_contract {
    //     res.push(vesting_claim_delegator_reward(mix_identity, fee, state).await?);
    // }
    // log::trace!("<<< {:?}", res);
    // Ok(res)
}
