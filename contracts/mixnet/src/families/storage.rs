use cosmwasm_std::{StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use mixnet_contract_common::families::{family_storage_key, Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKeyRef};

use crate::constants::{FAMILIES_INDEX_NAMESPACE, FAMILIES_MAP_NAMESPACE};

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

pub fn get_family(
    head: &FamilyHead,
    proxy: Option<String>,
    store: &dyn Storage,
) -> Result<Family, MixnetContractError> {
    let key = family_storage_key(head.identity(), proxy.as_ref());
    if let Some(family) = families().may_load(store, key)? {
        Ok(family)
    } else {
        Err(MixnetContractError::FamilyDoesNotExist {
            head: head.identity().to_string(),
            proxy: proxy.unwrap_or_default(),
        })
    }
}

pub fn create_family(f: &Family, store: &mut dyn Storage) -> Result<(), MixnetContractError> {
    match families().save(store, f.storage_key(), f) {
        Ok(()) => Ok(()),
        Err(e) => match &e {
            StdError::GenericErr { msg } => {
                if msg.starts_with("Violates unique constraint") {
                    Err(MixnetContractError::FamilyWithLabelExists(
                        f.label().to_string(),
                    ))
                } else {
                    Err(e.into())
                }
            }
            _ => Err(e.into()),
        },
    }
}

pub fn save_family(f: &Family, store: &mut dyn Storage) -> Result<(), MixnetContractError> {
    Ok(families().save(store, f.storage_key(), f)?)
}

pub fn add_family_member(
    f: &mut Family,
    store: &mut dyn Storage,
    member: IdentityKeyRef<'_>,
) -> Result<(), MixnetContractError> {
    f.members_mut().insert(member.to_string());
    save_family(f, store)
}

pub fn remove_family_member(
    f: &mut Family,
    store: &mut dyn Storage,
    member: IdentityKeyRef<'_>,
) -> Result<(), MixnetContractError> {
    f.members_mut().remove(member);
    save_family(f, store)
}
