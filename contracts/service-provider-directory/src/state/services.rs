use cosmwasm_std::{Addr, Coin, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use serde::{Deserialize, Serialize};

use crate::error::Result;

use crate::types::{Service, ServiceId};

const SERVICES_PK_NAMESPACE: &str = "sernames";
const SERVICES_OWNER_IDX_NAMESPACE: &str = "serown";
const SERVICES_NYM_ADDRESS_IDX_NAMESPACE: &str = "sernyma";

pub(crate) struct ServiceIndex<'a> {
    pub(crate) nym_address: MultiIndex<'a, String, Service, ServiceId>,
    pub(crate) owner: MultiIndex<'a, Addr, Service, ServiceId>,
}

impl<'a> IndexList<Service> for ServiceIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Service>> + '_> {
        let v: Vec<&dyn Index<Service>> = vec![&self.nym_address, &self.owner];
        Box::new(v.into_iter())
    }
}

pub(crate) fn services<'a>() -> IndexedMap<'a, ServiceId, Service, ServiceIndex<'a>> {
    let indexes = ServiceIndex {
        nym_address: MultiIndex::new(
            |d| d.nym_address.to_string(),
            SERVICES_PK_NAMESPACE,
            SERVICES_NYM_ADDRESS_IDX_NAMESPACE,
        ),
        owner: MultiIndex::new(
            |d| d.owner.clone(),
            SERVICES_PK_NAMESPACE,
            SERVICES_OWNER_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(SERVICES_PK_NAMESPACE, indexes)
}
