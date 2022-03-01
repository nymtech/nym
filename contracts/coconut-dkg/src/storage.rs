// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{BlockHeight, Epoch};
use coconut_dkg_common::types::{EncodedChannelPublicKey, IssuerDetails, NodeIndex};
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

pub(crate) const CURRENT_ISSUERS: Map<'_, Addr, IssuerDetails> = Map::new("cti");

// keep track of all validators that have left the IA set, so that we'd known when we have
// to create fresh set of keys; we also keep track of the block height of when they left
pub(crate) const INACTIVE_ISSUERS: Map<'_, Addr, (IssuerDetails, BlockHeight)> =
    Map::new("inactive");

pub(crate) const NODE_INDEX_COUNTER: Item<NodeIndex> = Item::new("node_index_counter");

pub(crate) const EPOCH_COUNTER: Item<Epoch> = Item::new("epoch_counter");

enum SharingType {
    TotalRegeneration,
    NewIssuer,
}

// presumably those should be moved to a different file?
#[derive(Serialize, Deserialize)]
pub(crate) struct ContractState {
    initial_exchange_height: BlockHeight,
}

impl ContractState {
    pub(crate) fn new(initial_exchange_height: BlockHeight) -> Self {
        ContractState {
            initial_exchange_height,
        }
    }
}

pub(crate) const CONTRACT_STATE: Item<ContractState> = Item::new("state");

// each issuer should receive q shares from N dealers

pub(crate) fn next_node_index(store: &mut dyn Storage) -> StdResult<NodeIndex> {
    let id: u64 = NODE_INDEX_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NODE_INDEX_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) fn submit_issuer(
    store: &mut dyn Storage,
    addr: Addr,
    encoded_key: EncodedChannelPublicKey,
) -> StdResult<NodeIndex> {
    let id = next_node_index(store)?;
    let issuer_details = IssuerDetails::new(encoded_key, id);
    CURRENT_ISSUERS.save(store, addr, &issuer_details)?;

    Ok(id)
}

// TODO: there also needs to be something for the partial coconut verification keys

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn first_node_index_is_one() {
        // essentially a sanity check that we NEVER start with 0
        let mut deps = mock_dependencies();
        let first = next_node_index(&mut deps.storage).unwrap();
        assert_eq!(first, 1);
    }
}
