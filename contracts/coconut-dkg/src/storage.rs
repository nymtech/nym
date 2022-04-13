// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{Blacklisting, DealerDetails, NodeIndex};
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

pub(crate) const BLACKLISTED_DEALERS: Map<'_, &'_ Addr, Blacklisting> = Map::new("bld");

pub(crate) const NODE_INDEX_COUNTER: Item<NodeIndex> = Item::new("node_index_counter");

// TODO: this is an interesting question: should dealers be addressed by their addresses
// or maybe node indices?
// perhaps this should be UniqueIndex?
pub(crate) const CURRENT_DEALERS: Map<'_, &'_ Addr, DealerDetails> = Map::new("crd");
pub(crate) const PAST_DEALERS: Map<'_, &'_ Addr, DealerDetails> = Map::new("ptd");

pub(crate) fn next_node_index(store: &mut dyn Storage) -> StdResult<NodeIndex> {
    // make sure we don't start from 0!
    let id: NodeIndex = NODE_INDEX_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NODE_INDEX_COUNTER.save(store, &id)?;
    Ok(id)
}
