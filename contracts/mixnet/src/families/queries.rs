use crate::constants::{FAMILIES_DEFAULT_RETRIEVAL_LIMIT, FAMILIES_MAX_RETRIEVAL_LIMIT};

use super::storage::{families, get_family, FAMILY_LAYERS};
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::families::{Family, FamilyAnnotated, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKeyRef};
use mixnet_contract_common::{Layer, PagedFamiliesResponse};

fn get_family_layer(
    storage: &dyn Storage,
    family: &Family,
) -> Result<Option<Layer>, MixnetContractError> {
    Ok(FAMILY_LAYERS.may_load(storage, family.storage_key())?)
}

pub fn get_family_by_label(
    label: &str,
    storage: &dyn Storage,
) -> Result<Option<FamilyAnnotated>, MixnetContractError> {
    let family = families()
        .idx
        .label
        .item(storage, label.to_string())?
        .map(|o| o.1);
    let layer = family
        .as_ref()
        .and_then(|f| get_family_layer(storage, f).ok())
        .flatten();
    Ok(family.map(|f| f.into_annotated(layer)))
}

pub fn get_family_by_head(
    head: IdentityKeyRef<'_>,
    proxy: Option<String>,
    storage: &dyn Storage,
) -> Result<FamilyAnnotated, MixnetContractError> {
    let family_head = FamilyHead::new(head);
    let family = get_family(&family_head, proxy, storage)?;
    let layer = get_family_layer(storage, &family)?;
    Ok(family.into_annotated(layer))
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

    let start_next_after = response.last().map(|response| response.storage_key());

    Ok(PagedFamiliesResponse {
        families: response,
        start_next_after,
    })
}
