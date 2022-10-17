use super::storage::{families, get_family};
use cosmwasm_std::{Order, Storage};
use mixnet_contract_common::families::{Family, FamilyHead};
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
