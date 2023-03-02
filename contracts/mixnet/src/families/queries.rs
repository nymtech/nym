use std::collections::HashSet;

use crate::constants::{FAMILIES_DEFAULT_RETRIEVAL_LIMIT, FAMILIES_MAX_RETRIEVAL_LIMIT};

use super::storage::{families, get_family, get_members, MEMBERS};
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKeyRef};
use mixnet_contract_common::{IdentityKey, PagedFamiliesResponse, PagedMembersResponse};

pub fn get_family_by_label(
    label: String,
    storage: &dyn Storage,
) -> Result<Option<Family>, MixnetContractError> {
    Ok(families().idx.label.item(storage, label)?.map(|o| o.1))
}

pub fn get_family_by_head(
    head: IdentityKeyRef<'_>,
    storage: &dyn Storage,
) -> Result<Family, MixnetContractError> {
    let family_head = FamilyHead::new(head);
    get_family(&family_head, storage)
}

pub fn get_family_members_by_head(
    head: IdentityKeyRef<'_>,
    storage: &dyn Storage,
) -> Result<HashSet<String>, MixnetContractError> {
    let family_head = FamilyHead::new(head);
    let family = get_family(&family_head, storage)?;
    get_members(&family, storage)
}

pub fn get_family_members_by_label(
    label: String,
    storage: &dyn Storage,
) -> Result<Option<HashSet<String>>, MixnetContractError> {
    if let Some(family) = families().idx.label.item(storage, label)?.map(|o| o.1) {
        Ok(Some(get_members(&family, storage)?))
    } else {
        Ok(None)
    }
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
