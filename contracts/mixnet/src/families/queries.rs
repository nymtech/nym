// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage::{families, get_members, must_get_family, MEMBERS};
use crate::constants::{FAMILIES_DEFAULT_RETRIEVAL_LIMIT, FAMILIES_MAX_RETRIEVAL_LIMIT};
use crate::families::storage::must_get_family_by_label;
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::families::{
    Family, FamilyByHeadResponse, FamilyByLabelResponse, FamilyHead, FamilyMembersByHeadResponse,
    PagedFamiliesResponse, PagedMembersResponse,
};
use mixnet_contract_common::{error::MixnetContractError, IdentityKeyRef};
use mixnet_contract_common::{FamilyMembersByLabelResponse, IdentityKey};

pub fn get_family_by_label(
    label: String,
    storage: &dyn Storage,
) -> Result<FamilyByLabelResponse, MixnetContractError> {
    let family = families()
        .idx
        .label
        .item(storage, label.clone())?
        .map(|o| o.1);
    Ok(FamilyByLabelResponse { label, family })
}

pub fn get_family_by_head(
    head: IdentityKeyRef<'_>,
    storage: &dyn Storage,
) -> Result<FamilyByHeadResponse, MixnetContractError> {
    let family = families().may_load(storage, head.to_string())?;
    Ok(FamilyByHeadResponse {
        head: FamilyHead::new(head),
        family,
    })
}

// TODO: this should be returning a paged response!
pub fn get_family_members_by_head(
    head: IdentityKeyRef<'_>,
    storage: &dyn Storage,
) -> Result<FamilyMembersByHeadResponse, MixnetContractError> {
    let family_head = FamilyHead::new(head);
    let family = must_get_family(&family_head, storage)?;
    let members = get_members(&family, storage)?;

    Ok(FamilyMembersByHeadResponse {
        head: family.head().to_owned(),
        members,
    })
}

// TODO: this should be returning a paged response!
pub fn get_family_members_by_label(
    label: String,
    storage: &dyn Storage,
) -> Result<FamilyMembersByLabelResponse, MixnetContractError> {
    let family = must_get_family_by_label(label.clone(), storage)?;
    let members = get_members(&family, storage)?;

    Ok(FamilyMembersByLabelResponse { label, members })
}

pub fn get_all_families_paged(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<PagedFamiliesResponse, MixnetContractError> {
    let limit = limit
        .unwrap_or(FAMILIES_DEFAULT_RETRIEVAL_LIMIT)
        .min(FAMILIES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let response = families()
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .filter_map(|r| r.ok())
        .map(|(_key, family)| family)
        .collect::<Vec<Family>>();

    let start_next_after = response
        .last()
        .map(|response| response.head_identity().to_string());

    Ok(PagedFamiliesResponse {
        families: response,
        start_next_after,
    })
}

pub fn get_all_members_paged(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<PagedMembersResponse, MixnetContractError> {
    let limit = limit
        .unwrap_or(FAMILIES_DEFAULT_RETRIEVAL_LIMIT)
        .min(FAMILIES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let response = MEMBERS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .filter_map(|r| r.ok())
        .collect::<Vec<(IdentityKey, FamilyHead)>>();

    let start_next_after = response.last().map(|r| r.0.clone());

    Ok(PagedMembersResponse {
        members: response,
        start_next_after,
    })
}
