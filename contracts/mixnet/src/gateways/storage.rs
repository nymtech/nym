// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    GATEWAYS_OWNER_IDX_NAMESPACE, GATEWAYS_PK_NAMESPACE, LEGACY_GATEWAY_ID_NAMESPACE,
};
use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, UniqueIndex};
use mixnet_contract_common::{GatewayBond, IdentityKeyRef, NodeId};
use nym_contracts_common::IdentityKey;

pub(crate) const PREASSIGNED_LEGACY_IDS: Map<IdentityKey, NodeId> =
    Map::new(LEGACY_GATEWAY_ID_NAMESPACE);

pub(crate) struct GatewayBondIndex<'a> {
    pub(crate) owner: UniqueIndex<'a, Addr, GatewayBond>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl IndexList<GatewayBond> for GatewayBondIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<GatewayBond>> + '_> {
        let v: Vec<&dyn Index<GatewayBond>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

// gateways() is the storage access function.
pub(crate) fn gateways<'a>() -> IndexedMap<'a, IdentityKeyRef<'a>, GatewayBond, GatewayBondIndex<'a>>
{
    let indexes = GatewayBondIndex {
        owner: UniqueIndex::new(|d| d.owner.clone(), GATEWAYS_OWNER_IDX_NAMESPACE),
    };
    IndexedMap::new(GATEWAYS_PK_NAMESPACE, indexes)
}
