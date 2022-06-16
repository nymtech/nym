use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::vesting::rewards::{vesting_claim_delegator_reward, vesting_compound_delegator_reward};
use mixnet_contract_common::IdentityKey;
use nym_types::transaction::TransactionExecuteResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn claim_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    // TODO: handle operator bonding with vesting contract
    log::info!(">>> Claim operator reward");
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_claim_operator_reward(fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn compound_operator_reward(
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    // TODO: handle operator bonding with vesting contract
    log::info!(">>> Compound operator reward");
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_compound_operator_reward(fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn claim_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(
        ">>> Claim delegator reward: identity_key = {}",
        mix_identity
    );
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_claim_delegator_reward(mix_identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn compound_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(
        ">>> Compound delegator reward: identity_key = {}",
        mix_identity
    );
    let denom_minor = state.read().await.current_network().denom();
    let res = nymd_client!(state)
        .execute_compound_delegator_reward(mix_identity, fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res,
        denom_minor.as_ref(),
    )?)
}

#[tauri::command]
pub async fn claim_locked_and_unlocked_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<TransactionExecuteResult>, BackendError> {
    log::info!(
        ">>> Claim delegator reward (locked and unlocked): identity_key = {}",
        mix_identity
    );

    log::trace!(">>> Get delegations: identity_key = {}", mix_identity);
    let address = nymd_client!(state).address().to_string();
    let delegations = nymd_client!(state)
        .get_delegator_delegations_paged(address.clone(), None, None) // get all delegations, ignoring paging
        .await?
        .delegations;
    log::trace!("<<< {} delegations", delegations.len());

    // decide which contracts to use, could be neither
    let did_delegate_with_mixnet_contract = delegations
        .iter()
        .filter(|f| f.node_identity == mix_identity)
        .any(|f| f.proxy.is_none());
    let did_delegate_with_vesting_contract = delegations
        .iter()
        .filter(|f| f.node_identity == mix_identity)
        .any(|f| f.proxy.is_some());

    log::trace!(
        "<<< Delegations done with: mixnet contract = {}, vesting contract = {}",
        did_delegate_with_mixnet_contract,
        did_delegate_with_vesting_contract
    );

    let mut res: Vec<TransactionExecuteResult> = vec![];
    if did_delegate_with_mixnet_contract {
        res.push(claim_delegator_reward(mix_identity.clone(), fee.clone(), state.clone()).await?);
    }
    if did_delegate_with_vesting_contract {
        res.push(vesting_claim_delegator_reward(mix_identity, fee, state).await?);
    }
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn compound_locked_and_unlocked_delegator_reward(
    mix_identity: IdentityKey,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<TransactionExecuteResult>, BackendError> {
    log::info!(
        ">>> Compound delegator reward (locked and unlocked): identity_key = {}",
        mix_identity
    );

    log::trace!(">>> Get delegations: identity_key = {}", mix_identity);
    let address = nymd_client!(state).address().to_string();
    let delegations = nymd_client!(state)
        .get_delegator_delegations_paged(address.clone(), None, None) // get all delegations, ignoring paging
        .await?
        .delegations;
    log::trace!("<<< {} delegations", delegations.len());

    // decide which contracts to use, could be neither
    let did_delegate_with_mixnet_contract = delegations
        .iter()
        .filter(|f| f.node_identity == mix_identity)
        .any(|f| f.proxy.is_none());
    let did_delegate_with_vesting_contract = delegations
        .iter()
        .filter(|f| f.node_identity == mix_identity)
        .any(|f| f.proxy.is_some());

    log::trace!(
        "<<< Delegations done with: mixnet contract = {}, vesting contract = {}",
        did_delegate_with_mixnet_contract,
        did_delegate_with_vesting_contract
    );

    let mut res: Vec<TransactionExecuteResult> = vec![];
    if did_delegate_with_mixnet_contract {
        res.push(
            compound_delegator_reward(mix_identity.clone(), fee.clone(), state.clone()).await?,
        );
    }
    if did_delegate_with_vesting_contract {
        res.push(vesting_compound_delegator_reward(mix_identity, fee, state).await?);
    }
    log::trace!("<<< {:?}", res);
    Ok(res)
}
