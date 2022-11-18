use crate::constants::{FAMILIES_DEFAULT_RETRIEVAL_LIMIT, FAMILIES_MAX_RETRIEVAL_LIMIT};

use super::storage::{families, get_family};
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::PagedFamiliesResponse;
use mixnet_contract_common::{error::MixnetContractError, IdentityKeyRef};

pub fn get_family_by_label(
    label: &str,
    storage: &dyn Storage,
) -> Result<Option<Family>, MixnetContractError> {
    Ok(families()
        .idx
        .label
        .item(storage, label.to_string())?
        .map(|o| o.1))
}

pub fn get_family_by_head(
    head: IdentityKeyRef<'_>,
    proxy: Option<String>,
    storage: &dyn Storage,
) -> Result<Family, MixnetContractError> {
    let family_head = FamilyHead::new(head);
    get_family(&family_head, proxy, storage)
}

pub fn get_all_families(storage: &dyn Storage) -> Vec<Family> {
    families()
        .range(storage, None, None, Order::Ascending)
        .filter_map(|f| f.ok())
        .map(|(_head, family)| family)
        .collect::<Vec<Family>>()
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
        .map(|(key, family)| family)
        .collect::<Vec<Family>>();

    let start_next_after = response.last().map(|response| response.storage_key());

    Ok(PagedFamiliesResponse {
        families: response,
        start_next_after,
    })
}
