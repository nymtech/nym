use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey, IdentityKeyRef};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
        label: UniqueIndex::new(|d| d.label.clone(), "faml"),
    };
    IndexedMap::new("fam", indexes)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Family {
    head: FamilyHead,
    proxy: Option<Addr>,
    label: String,
    members: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FamilyHead(IdentityKey);

impl FamilyHead {
    pub fn new(identity: IdentityKeyRef<'_>) -> Self {
        FamilyHead(identity.to_string())
    }

    pub fn identity(&self) -> IdentityKeyRef<'_> {
        &self.0
    }
}

impl Family {
    fn _new(head: FamilyHead, proxy: Option<Addr>, label: String) -> Self {
        Family {
            head,
            proxy,
            label,
            members: HashSet::new(),
        }
    }

    pub fn new(
        head: FamilyHead,
        proxy: Option<Addr>,
        label: &str,
        store: &mut dyn Storage,
    ) -> Result<Family, MixnetContractError> {
        let family = Family::_new(head, proxy, label.to_string());
        family.create(store)?;
        Ok(family)
    }

    pub fn get(
        head: &FamilyHead,
        proxy: Option<Addr>,
        store: &dyn Storage,
    ) -> Result<Family, MixnetContractError> {
        let key = storage_key(head.identity(), proxy.as_ref());
        Ok(families().load(store, key)?)
    }

    #[allow(dead_code)]
    pub fn head(&self) -> &FamilyHead {
        &self.head
    }

    pub fn head_identity(&self) -> IdentityKeyRef<'_> {
        self.head.identity()
    }

    #[allow(dead_code)]
    pub fn proxy(&self) -> Option<&Addr> {
        self.proxy.as_ref()
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[allow(dead_code)]
    pub fn members(&self) -> &HashSet<String> {
        &self.members
    }

    pub fn members_mut(&mut self) -> &mut HashSet<String> {
        &mut self.members
    }

    pub fn storage_key(&self) -> String {
        storage_key(self.head_identity(), self.proxy.as_ref())
    }

    pub fn create(&self, store: &mut dyn Storage) -> Result<(), MixnetContractError> {
        Ok(families().save(store, self.storage_key(), self)?)
    }

    pub fn save(&self, store: &mut dyn Storage) -> Result<(), MixnetContractError> {
        Ok(families().save(store, self.storage_key(), self)?)
    }

    #[allow(dead_code)]
    pub fn is_member(&self, member: IdentityKeyRef<'_>) -> bool {
        self.members().contains(member)
    }

    pub fn add(
        &mut self,
        store: &mut dyn Storage,
        member: IdentityKeyRef<'_>,
    ) -> Result<(), MixnetContractError> {
        self.members_mut().insert(member.to_string());
        self.save(store)
    }

    pub fn remove(
        &mut self,
        store: &mut dyn Storage,
        member: IdentityKeyRef<'_>,
    ) -> Result<(), MixnetContractError> {
        self.members_mut().remove(member);
        self.save(store)
    }
}

fn storage_key(head: &str, proxy: Option<&Addr>) -> String {
    if let Some(proxy) = proxy {
        let key_bytes = head
            .as_bytes()
            .iter()
            .zip(proxy.as_bytes())
            .map(|(x, y)| x ^ y)
            .collect::<Vec<_>>();
        bs58::encode(key_bytes).into_string()
    } else {
        head.to_string()
    }
}
