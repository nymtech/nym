// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Dealer;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};
use nym_coconut_dkg_common::types::{DealerDetails, DealerRegistrationDetails, EpochId, NodeIndex};

const CURRENT_DEALERS_PK: &str = "crd";
const PAST_DEALERS_PK: &str = "ptd";
const DEALERS_NODE_INDEX_IDX_NAMESPACE: &str = "dni";

pub(crate) const DEALERS_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const DEALERS_PAGE_DEFAULT_LIMIT: u32 = 50;

pub(crate) const NODE_INDEX_COUNTER: Item<NodeIndex> = Item::new("node_index_counter");

// TODO:
pub(crate) const DEALERS_INDICES: Map<Dealer, NodeIndex> = Map::new("dealer_index");
pub(crate) const EPOCH_DEALERS_MAP: Map<(EpochId, Dealer), DealerRegistrationDetails> =
    Map::new("epoch_dealers");

pub(crate) type IndexedDealersMap<'a> = IndexedMap<'a, &'a Addr, DealerDetails, DealersIndex<'a>>;

pub(crate) struct DealersIndex<'a> {
    pub(crate) node_index: UniqueIndex<'a, NodeIndex, DealerDetails>,
}

impl<'a> IndexList<DealerDetails> for DealersIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<DealerDetails>> + '_> {
        let v: Vec<&dyn Index<DealerDetails>> = vec![&self.node_index];
        Box::new(v.into_iter())
    }
}

pub(crate) fn current_dealers<'a>() -> IndexedDealersMap<'a> {
    let indexes = DealersIndex {
        node_index: UniqueIndex::new(|d| d.assigned_index, DEALERS_NODE_INDEX_IDX_NAMESPACE),
    };
    IndexedMap::new(CURRENT_DEALERS_PK, indexes)
}

pub(crate) fn past_dealers<'a>() -> IndexedDealersMap<'a> {
    let indexes = DealersIndex {
        node_index: UniqueIndex::new(|d| d.assigned_index, DEALERS_NODE_INDEX_IDX_NAMESPACE),
    };
    IndexedMap::new(PAST_DEALERS_PK, indexes)
}

pub(crate) fn next_node_index(store: &mut dyn Storage) -> StdResult<NodeIndex> {
    // make sure we don't start from 0, otherwise all the crypto breaks (kinda)
    let id: NodeIndex = NODE_INDEX_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NODE_INDEX_COUNTER.save(store, &id)?;
    Ok(id)
}
