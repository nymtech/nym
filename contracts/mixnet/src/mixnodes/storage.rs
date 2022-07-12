// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    LAYER_DISTRIBUTION_KEY, MIXNODES_IDENTITY_IDX_NAMESPACE, MIXNODES_OWNER_IDX_NAMESPACE,
    MIXNODES_PK_NAMESPACE, MIXNODES_SPHINX_IDX_NAMESPACE, NODE_ID_COUNTER_KEY,
    UNBONDED_MIXNODES_PK_NAMESPACE,
};
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::UnbondedMixnode;
use mixnet_contract_common::SphinxKey;
use mixnet_contract_common::{Addr, IdentityKey, Layer, LayerDistribution, MixNodeBond, NodeId};

// storage prefixes

// // paged retrieval limits for all queries and transactions
// pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 75;
// pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;
//
//

// keeps track of `node_id -> IdentityKey` so we'd known a bit more about past mixnodes
// if we ever decide it's too bloaty, we can deprecate it and start removing all data in
// subsequent migrations
pub(crate) const UNBONDED_MIXNODES: Map<NodeId, UnbondedMixnode> =
    Map::new(UNBONDED_MIXNODES_PK_NAMESPACE);

pub(crate) const LAYERS: Item<'_, LayerDistribution> = Item::new(LAYER_DISTRIBUTION_KEY);
pub const MIXNODE_ID_COUNTER: Item<NodeId> = Item::new(NODE_ID_COUNTER_KEY);

// mixnode_bonds() is the storage access function.
pub(crate) fn mixnode_bonds<'a>() -> IndexedMap<'a, NodeId, MixNodeBond, MixnodeBondIndex<'a>> {
    let indexes = MixnodeBondIndex {
        owner: UniqueIndex::new(|d| d.owner.clone(), MIXNODES_OWNER_IDX_NAMESPACE),
        identity_key: UniqueIndex::new(
            |d| d.mix_node.identity_key.clone(),
            MIXNODES_IDENTITY_IDX_NAMESPACE,
        ),
        sphinx_key: UniqueIndex::new(
            |d| d.mix_node.sphinx_key.clone(),
            MIXNODES_SPHINX_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(MIXNODES_PK_NAMESPACE, indexes)
}

pub fn decrement_layer_count(
    storage: &mut dyn Storage,
    layer: Layer,
) -> Result<(), MixnetContractError> {
    let mut layers = LAYERS.load(storage)?;
    layers.decrement_layer_count(layer)?;
    Ok(LAYERS.save(storage, &layers)?)
}

pub(crate) fn assign_layer(store: &mut dyn Storage) -> StdResult<Layer> {
    // load current distribution
    let mut layers = LAYERS.load(store)?;

    // choose the one with fewest nodes
    let fewest = layers.choose_with_fewest();

    // increment the existing count
    layers.increment_layer_count(fewest);

    // and resave it
    LAYERS.save(store, &layers)?;
    Ok(fewest)
}

pub(crate) fn next_mixnode_id_counter(store: &mut dyn Storage) -> StdResult<NodeId> {
    let id: NodeId = MIXNODE_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    MIXNODE_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) struct MixnodeBondIndex<'a> {
    pub(crate) owner: UniqueIndex<'a, Addr, MixNodeBond>,

    pub(crate) identity_key: UniqueIndex<'a, IdentityKey, MixNodeBond>,

    pub(crate) sphinx_key: UniqueIndex<'a, SphinxKey, MixNodeBond>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<MixNodeBond> for MixnodeBondIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<MixNodeBond>> + '_> {
        let v: Vec<&dyn Index<MixNodeBond>> =
            vec![&self.owner, &self.identity_key, &self.sphinx_key];
        Box::new(v.into_iter())
    }
}

//
// #[cfg(test)]
// mod tests {
//     use super::super::storage;
//     use super::*;
//     use crate::support::tests;
//     use config::defaults::MIX_DENOM;
//     use cosmwasm_std::testing::MockStorage;
//     use cosmwasm_std::{coin, Addr, Uint128};
//     use mixnet_contract_common::{IdentityKey, MixNode};
//
//     #[test]
//     fn mixnode_single_read_retrieval() {
//         let mut storage = MockStorage::new();
//         let bond1 = tests::fixtures::stored_mixnode_bond_fixture("owner1");
//         let bond2 = tests::fixtures::stored_mixnode_bond_fixture("owner2");
//         mixnodes().save(&mut storage, "bond1", &bond1, 1).unwrap();
//         mixnodes().save(&mut storage, "bond2", &bond2, 1).unwrap();
//
//         let res1 = mixnodes().load(&storage, "bond1").unwrap();
//         let res2 = mixnodes().load(&storage, "bond2").unwrap();
//         assert_eq!(bond1, res1);
//         assert_eq!(bond2, res2);
//     }
//
//     #[test]
//     fn reading_mixnode_bond() {
//         let mut mock_storage = MockStorage::new();
//         let node_owner: Addr = Addr::unchecked("node-owner");
//         let node_identity: IdentityKey = "nodeidentity".into();
//
//         // produces a None if target mixnode doesn't exist
//         let res = storage::read_full_mixnode_bond(&mock_storage, &node_identity).unwrap();
//         assert!(res.is_none());
//
//         // returns appropriate value otherwise
//         let pledge_value = 1000000000;
//
//         let mixnode_bond = StoredMixnodeBond {
//             pledge_amount: coin(pledge_value, MIX_DENOM.base),
//             owner: node_owner,
//             layer: Layer::One,
//             block_height: 12_345,
//             mix_node: MixNode {
//                 identity_key: node_identity.clone(),
//                 ..tests::fixtures::mix_node_fixture()
//             },
//             proxy: None,
//             accumulated_rewards: None,
//             epoch_rewards: None,
//         };
//
//         storage::mixnodes()
//             .save(&mut mock_storage, &node_identity, &mixnode_bond, 1)
//             .unwrap();
//
//         assert_eq!(
//             Uint128::new(pledge_value),
//             storage::read_full_mixnode_bond(&mock_storage, node_identity.as_str())
//                 .unwrap()
//                 .unwrap()
//                 .pledge_amount
//                 .amount
//         );
//     }
// }
