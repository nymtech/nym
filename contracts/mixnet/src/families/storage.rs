// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, UniqueIndex};
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey, IdentityKeyRef};
use std::collections::HashSet;

use crate::constants::{FAMILIES_INDEX_NAMESPACE, FAMILIES_MAP_NAMESPACE, MEMBERS_MAP_NAMESPACE};

pub struct FamilyIndex<'a> {
    pub label: UniqueIndex<'a, String, Family>,
}

impl<'a> IndexList<Family> for FamilyIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Family>> + '_> {
        let v: Vec<&dyn Index<Family>> = vec![&self.label];
        Box::new(v.into_iter())
    }
}

// storage access function.
pub fn families<'a>() -> IndexedMap<'a, String, Family, FamilyIndex<'a>> {
    let indexes = FamilyIndex {
        label: UniqueIndex::new(|d| d.label().to_string(), FAMILIES_INDEX_NAMESPACE),
    };
    IndexedMap::new(FAMILIES_MAP_NAMESPACE, indexes)
}

pub const MEMBERS: Map<IdentityKey, FamilyHead> = Map::new(MEMBERS_MAP_NAMESPACE);

pub fn get_members(
    family: &Family,
    store: &dyn Storage,
) -> Result<HashSet<IdentityKey>, MixnetContractError> {
    Ok(MEMBERS
        .range(store, None, None, Order::Ascending)
        .filter_map(|res| res.ok())
        .filter(|(_member, head)| head == family.head())
        .map(|(member, _storage_key)| member)
        .collect())
}

pub fn get_family(head: &FamilyHead, store: &dyn Storage) -> Result<Family, MixnetContractError> {
    let key = head.identity();
    if let Some(family) = families().may_load(store, key.to_string())? {
        Ok(family)
    } else {
        Err(MixnetContractError::FamilyDoesNotExist {
            head: head.identity().to_string(),
        })
    }
}

pub fn save_family(f: &Family, store: &mut dyn Storage) -> Result<(), MixnetContractError> {
    Ok(families().save(store, f.head_identity().to_string(), f)?)
}

pub fn add_family_member(
    f: &Family,
    store: &mut dyn Storage,
    member: IdentityKeyRef<'_>,
) -> Result<(), MixnetContractError> {
    Ok(MEMBERS.save(store, member.to_string(), f.head())?)
}

pub fn remove_family_member(store: &mut dyn Storage, member: IdentityKeyRef<'_>) {
    MEMBERS.remove(store, member.to_string())
}

pub fn is_family_member(
    store: &dyn Storage,
    f: &Family,
    member: IdentityKeyRef<'_>,
) -> Result<bool, MixnetContractError> {
    let existing_head = MEMBERS.may_load(store, member.to_owned())?;
    Ok(existing_head.as_ref() == Some(f.head()))
}

pub fn is_any_member(
    store: &dyn Storage,
    member: IdentityKeyRef<'_>,
) -> Result<Option<FamilyHead>, MixnetContractError> {
    Ok(MEMBERS.may_load(store, member.to_string())?)
}
