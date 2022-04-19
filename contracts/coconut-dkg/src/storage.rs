// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{
    Blacklisting, BlacklistingReason, BlockHeight, DealerDetails, NodeIndex,
};
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

const CURRENT_DEALERS_PK: &str = "crd";
const PAST_DEALERS_PK: &str = "ptd";
const DEALERS_NODE_INDEX_IDX_NAMESPACE: &str = "dni";

pub(crate) const BLACKLISTED_DEALERS: Map<'_, &'_ Addr, Blacklisting> = Map::new("bld");
pub(crate) const NODE_INDEX_COUNTER: Item<NodeIndex> = Item::new("node_index_counter");

pub(crate) struct DealersIndex<'a> {
    pub(crate) node_index: UniqueIndex<'a, NodeIndex, DealerDetails>,
}

impl<'a> IndexList<DealerDetails> for DealersIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<DealerDetails>> + '_> {
        let v: Vec<&dyn Index<DealerDetails>> = vec![&self.node_index];
        Box::new(v.into_iter())
    }
}

pub(crate) fn current_dealers<'a>() -> IndexedMap<'a, &'a Addr, DealerDetails, DealersIndex<'a>> {
    let indexes = DealersIndex {
        node_index: UniqueIndex::new(|d| d.assigned_index, DEALERS_NODE_INDEX_IDX_NAMESPACE),
    };
    IndexedMap::new(CURRENT_DEALERS_PK, indexes)
}

pub(crate) fn past_dealers<'a>() -> IndexedMap<'a, &'a Addr, DealerDetails, DealersIndex<'a>> {
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

pub(crate) fn blacklist_dealer(
    store: &mut dyn Storage,
    dealer: &Addr,
    reason: BlacklistingReason,
    current_block_height: BlockHeight,
    expiration: Option<BlockHeight>,
) -> StdResult<()> {
    let blacklisting = Blacklisting {
        reason,
        height: current_block_height,
        expiration,
    };
    BLACKLISTED_DEALERS.save(store, dealer, &blacklisting)
}

pub(crate) fn obtain_blacklisting(
    store: &mut dyn Storage,
    dealer: &Addr,
    current_height: BlockHeight,
) -> StdResult<Option<(Blacklisting, bool)>> {
    if let Some(blacklisting) = BLACKLISTED_DEALERS.may_load(store, dealer)? {
        if !blacklisting.has_expired(current_height) {
            Ok(Some((blacklisting, false)))
        } else {
            // remove the blacklisting if it has expired
            BLACKLISTED_DEALERS.remove(store, dealer);
            Ok(Some((blacklisting, true)))
        }
    } else {
        Ok(None)
    }
}
