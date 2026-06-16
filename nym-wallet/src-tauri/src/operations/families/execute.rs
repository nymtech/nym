// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! State-changing node-families commands over [`NodeFamiliesSigningClient`].
//!
//! Every command uses the wallet's standard auto/simulated gas fee convention
//! (design D7): the frontend bindings don't supply an explicit gas `Fee`, so we
//! pass `None` and let the client simulate. `create_family` additionally attaches
//! the configured `create_family_fee` (a display [`DecCoin`] from
//! `get_family_config`) as funds, converted back to its base denomination.
//!
//! The returned [`TransactionExecuteResult`] is a subset of the frontend's
//! `FamilyTxResult` (which adds an optional `family_events`). Per design D2 we
//! omit `family_events` — the provider re-derives state via `refreshAll()` after
//! every execute, and nothing reads the fabricated events.

use crate::error::BackendError;
use crate::state::WalletState;
use nym_mixnet_contract_common::NodeId;
use nym_node_families_contract_common::NodeFamilyId;
use nym_types::currency::DecCoin;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::contract_traits::NodeFamiliesSigningClient;

#[tauri::command]
pub async fn create_family(
    name: String,
    description: String,
    fee: DecCoin,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Create family: name = {name}, creation_fee = {fee}");
    let guard = state.read().await;
    // `fee` here is the contract's `create_family_fee` (attached as funds), not a gas fee.
    let creation_fee = vec![guard.attempt_convert_to_base_coin(fee)?];
    let res = guard
        .current_client()?
        .nyxd
        .create_family(name, description, None, creation_fee)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {res:?}");
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn update_family(
    updated_name: Option<String>,
    updated_description: Option<String>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Update family: name = {updated_name:?}, description = {updated_description:?}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .update_family(updated_name, updated_description, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn disband_family(
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Disband family");
    let guard = state.read().await;
    let res = guard.current_client()?.nyxd.disband_family(None).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn invite_to_family(
    node_id: NodeId,
    validity_secs: Option<u64>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Invite to family: node_id = {node_id}, validity_secs = {validity_secs:?}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .invite_to_family(node_id, validity_secs, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn revoke_family_invitation(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Revoke family invitation: node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .revoke_family_invitation(node_id, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn kick_from_family(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Kick from family: node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .kick_from_family(node_id, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn accept_family_invitation(
    family_id: NodeFamilyId,
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Accept family invitation: family_id = {family_id}, node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .accept_family_invitation(family_id, node_id, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn reject_family_invitation(
    family_id: NodeFamilyId,
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Reject family invitation: family_id = {family_id}, node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .reject_family_invitation(family_id, node_id, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}

#[tauri::command]
pub async fn leave_family(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> Leave family: node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .leave_family(node_id, None)
        .await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}
