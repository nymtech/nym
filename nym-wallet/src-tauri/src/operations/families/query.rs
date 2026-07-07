// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Read-only node-families commands over [`NodeFamiliesQueryClient`].
//!
//! The contract's paged responses each carry an echoed key (`family_id` /
//! `node_id`) plus a named items field (`members` / `invitations`) and a
//! `start_next_after` cursor. The frontend bindings expect a uniform
//! `{ items, start_next_after }` envelope ([`FamilyPagedResponse`]), so each
//! command flattens the contract response into it. Per the `src/types/families.ts`
//! contract — "the request layer is responsible for translating the contract's
//! serde envelope into these shapes" — we also normalise the cw_serde tagged
//! `FamilyInvitationStatus` enum into the wallet's `{ kind, at }` union.

use crate::error::BackendError;
use crate::state::WalletState;
use crate::state::WalletStateInner;
use nym_mixnet_contract_common::NodeId;
use nym_node_families_contract_common::{
    Config, FamilyInvitation, FamilyInvitationStatus, NodeFamily, NodeFamilyId,
    NodeFamilyMembershipResponse, PastFamilyInvitation, PastFamilyInvitationCursor,
    PastFamilyMember, PastFamilyMemberCursor, PendingFamilyInvitationDetails,
};
use nym_types::currency::DecCoin;
use nym_validator_client::nyxd::contract_traits::{NodeFamiliesQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{AccountId, Coin, CosmWasmClient};
use serde::Serialize;
use std::str::FromStr;

/// Uniform `{ items, start_next_after }` envelope the frontend's
/// `FamilyPagedResponse<T>` expects. `C` is the (page-specific) cursor type:
/// a `NodeId`/`NodeFamilyId` for single-key pages or a `(node_id, counter)`
/// tuple for the archived listings.
#[derive(Serialize)]
pub struct FamilyPagedResponse<T, C> {
    pub items: Vec<T>,
    pub start_next_after: Option<C>,
}

/// One current-member row: `{ node_id, joined_at }` (drops the contract's
/// nested `membership` envelope the UI doesn't use here).
#[derive(Serialize)]
pub struct FamilyMemberItem {
    pub node_id: NodeId,
    pub joined_at: u64,
}

/// Wallet-friendly `{ kind, at }` form of the contract's tagged
/// `FamilyInvitationStatus` enum.
#[derive(Serialize)]
pub struct PastInvitationStatus {
    pub kind: String,
    pub at: u64,
}

/// `PastFamilyInvitation` with its status normalised for the frontend.
#[derive(Serialize)]
pub struct PastFamilyInvitationItem {
    pub invitation: FamilyInvitation,
    pub status: PastInvitationStatus,
}

/// Frontend `NodeFamily`: the contract's `paid_fee` (stored in the base
/// denomination) is converted to a display [`DecCoin`] and `owner` to a plain
/// string, so the UI can `formatCoin(paid_fee)` directly.
#[derive(Serialize)]
pub struct NodeFamilyView {
    pub id: NodeFamilyId,
    pub name: String,
    pub description: String,
    pub normalised_name: String,
    pub members: u64,
    pub created_at: u64,
    pub paid_fee: DecCoin,
    pub owner: String,
}

/// Frontend `FamilyConfig`: `create_family_fee` returned as a display
/// [`DecCoin`] (converted from the contract's base-denom `Coin`) so the UI can
/// round-trip it straight back into `create_family`.
#[derive(Serialize)]
pub struct FamilyConfigResponse {
    pub create_family_fee: DecCoin,
    pub family_name_length_limit: usize,
    pub family_description_length_limit: usize,
    pub default_invitation_validity_secs: u64,
}

fn normalise_status(status: FamilyInvitationStatus) -> PastInvitationStatus {
    let (kind, at) = match status {
        FamilyInvitationStatus::Pending { at } => ("Pending", at),
        FamilyInvitationStatus::Accepted { at } => ("Accepted", at),
        FamilyInvitationStatus::Rejected { at } => ("Rejected", at),
        FamilyInvitationStatus::Revoked { at } => ("Revoked", at),
        FamilyInvitationStatus::Expired { at } => ("Expired", at),
    };
    PastInvitationStatus {
        kind: kind.to_string(),
        at,
    }
}

fn map_past_invitation(past: PastFamilyInvitation) -> PastFamilyInvitationItem {
    PastFamilyInvitationItem {
        invitation: past.invitation,
        status: normalise_status(past.status),
    }
}

fn map_family(
    state: &WalletStateInner,
    family: NodeFamily,
) -> Result<NodeFamilyView, BackendError> {
    Ok(NodeFamilyView {
        id: family.id,
        name: family.name,
        description: family.description,
        normalised_name: family.normalised_name,
        members: family.members,
        created_at: family.created_at,
        paid_fee: state.attempt_convert_to_display_dec_coin(Coin::from(family.paid_fee))?,
        owner: family.owner.to_string(),
    })
}

// --- Single-entity queries --------------------------------------------------

#[tauri::command]
pub async fn get_family_by_id(
    family_id: NodeFamilyId,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<NodeFamilyView>, BackendError> {
    log::trace!(">>> Get family by id: family_id = {family_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_family_by_id(family_id)
        .await?;
    res.family.map(|f| map_family(&guard, f)).transpose()
}

#[tauri::command]
pub async fn get_family_by_owner(
    owner: String,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<NodeFamilyView>, BackendError> {
    log::trace!(">>> Get family by owner: owner = {owner}");
    let owner_addr = AccountId::from_str(&owner)?;
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_family_by_owner(&owner_addr)
        .await?;
    res.family.map(|f| map_family(&guard, f)).transpose()
}

#[tauri::command]
pub async fn get_family_membership(
    node_id: NodeId,
    state: tauri::State<'_, WalletState>,
) -> Result<NodeFamilyMembershipResponse, BackendError> {
    log::trace!(">>> Get family membership: node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_family_membership(node_id)
        .await?;
    Ok(res)
}

/// The contract exposes no `GetConfig` smart query (design D5), so we read the
/// `Config` `Item` from raw contract state at its storage key (`"config"`).
/// `create_family_fee` is stored in the base denomination; we convert it to a
/// display `DecCoin` for the UI.
#[tauri::command]
pub async fn get_family_config(
    state: tauri::State<'_, WalletState>,
) -> Result<FamilyConfigResponse, BackendError> {
    log::trace!(">>> Get family config");
    let guard = state.read().await;
    let client = guard.current_client()?;
    let contract = client
        .nyxd
        .node_families_contract_address()
        .ok_or_else(|| NyxdError::unavailable_contract_address("node families contract"))?
        .clone();
    let raw = client
        .nyxd
        .query_contract_raw(&contract, b"config".to_vec())
        .await?;
    let config: Config = serde_json::from_slice(&raw)?;
    let create_family_fee =
        guard.attempt_convert_to_display_dec_coin(Coin::from(config.create_family_fee))?;
    Ok(FamilyConfigResponse {
        create_family_fee,
        family_name_length_limit: config.family_name_length_limit,
        family_description_length_limit: config.family_description_length_limit,
        default_invitation_validity_secs: config.default_invitation_validity_secs,
    })
}

// --- Paginated queries ------------------------------------------------------

#[tauri::command]
pub async fn get_family_members_paged(
    family_id: NodeFamilyId,
    start_after: Option<NodeId>,
    limit: Option<u32>,
    state: tauri::State<'_, WalletState>,
) -> Result<FamilyPagedResponse<FamilyMemberItem, NodeId>, BackendError> {
    log::trace!(">>> Get family members paged: family_id = {family_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_family_members_paged(family_id, start_after, limit)
        .await?;
    let items = res
        .members
        .into_iter()
        .map(|m| FamilyMemberItem {
            node_id: m.node_id,
            joined_at: m.membership.joined_at,
        })
        .collect();
    Ok(FamilyPagedResponse {
        items,
        start_next_after: res.start_next_after,
    })
}

#[tauri::command]
pub async fn get_pending_invitations_for_family_paged(
    family_id: NodeFamilyId,
    start_after: Option<NodeId>,
    limit: Option<u32>,
    state: tauri::State<'_, WalletState>,
) -> Result<FamilyPagedResponse<PendingFamilyInvitationDetails, NodeId>, BackendError> {
    log::trace!(">>> Get pending invitations for family paged: family_id = {family_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_pending_invitations_for_family_paged(family_id, start_after, limit)
        .await?;
    Ok(FamilyPagedResponse {
        items: res.invitations,
        start_next_after: res.start_next_after,
    })
}

#[tauri::command]
pub async fn get_pending_invitations_for_node_paged(
    node_id: NodeId,
    start_after: Option<NodeFamilyId>,
    limit: Option<u32>,
    state: tauri::State<'_, WalletState>,
) -> Result<FamilyPagedResponse<PendingFamilyInvitationDetails, NodeFamilyId>, BackendError> {
    log::trace!(">>> Get pending invitations for node paged: node_id = {node_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_pending_invitations_for_node_paged(node_id, start_after, limit)
        .await?;
    Ok(FamilyPagedResponse {
        items: res.invitations,
        start_next_after: res.start_next_after,
    })
}

#[tauri::command]
pub async fn get_past_invitations_for_family_paged(
    family_id: NodeFamilyId,
    start_after: Option<PastFamilyInvitationCursor>,
    limit: Option<u32>,
    state: tauri::State<'_, WalletState>,
) -> Result<FamilyPagedResponse<PastFamilyInvitationItem, PastFamilyInvitationCursor>, BackendError>
{
    log::trace!(">>> Get past invitations for family paged: family_id = {family_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_past_invitations_for_family_paged(family_id, start_after, limit)
        .await?;
    let items = res
        .invitations
        .into_iter()
        .map(map_past_invitation)
        .collect();
    Ok(FamilyPagedResponse {
        items,
        start_next_after: res.start_next_after,
    })
}

#[tauri::command]
pub async fn get_past_members_for_family_paged(
    family_id: NodeFamilyId,
    start_after: Option<PastFamilyMemberCursor>,
    limit: Option<u32>,
    state: tauri::State<'_, WalletState>,
) -> Result<FamilyPagedResponse<PastFamilyMember, PastFamilyMemberCursor>, BackendError> {
    log::trace!(">>> Get past members for family paged: family_id = {family_id}");
    let guard = state.read().await;
    let res = guard
        .current_client()?
        .nyxd
        .get_past_members_for_family_paged(family_id, start_after, limit)
        .await?;
    Ok(FamilyPagedResponse {
        items: res.members,
        start_next_after: res.start_next_after,
    })
}
