// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::MIX_DENOM;
use cosmwasm_std::{Decimal, Env, StdResult, Storage, Uint128};
use cw_storage_plus::{
    Index, IndexList, IndexedMap, IndexedSnapshotMap, Item, Map, Strategy, UniqueIndex,
};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::{
    MixNodeCostParams, MixNodeDetails, MixNodeRewarding, Period,
};
use mixnet_contract_common::rewarding::HistoricalRewards;
use mixnet_contract_common::{
    Addr, Coin, EpochId, IdentityKey, IdentityKeyRef, Layer, LayerDistribution, MixNode,
    MixNodeBond, NodeId,
};
use mixnet_contract_common::{SphinxKey, U128};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

// storage prefixes
const MIXNODES_PK_NAMESPACE: &str = "mnn";
const MIXNODES_OWNER_IDX_NAMESPACE: &str = "mno";
const MIXNODES_IDENTITY_IDX_NAMESPACE: &str = "mni";
const MIXNODES_SPHINX_IDX_NAMESPACE: &str = "mns";

const MIXNODES_REWARDING_PK_NAMESPACE: &str = "mnr";
const MIXNODES_HISTORICAL_RECORDS_PK_NAMESPACE: &str = "mnh";

// // paged retrieval limits for all queries and transactions
// pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 75;
// pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;
//
//

pub(crate) const LAYERS: Item<'_, LayerDistribution> = Item::new("layers");

// pub fn increment_layer_count(
//     storage: &mut dyn Storage,
//     layer: Layer,
// ) -> Result<(), MixnetContractError> {
//     let mut layers = LAYERS.load(storage)?;
//     layers.increment_layer_count(layer);
//     Ok(LAYERS.save(storage, &layers)?)
// }

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

pub const MIXNODE_ID_COUNTER: Item<NodeId> = Item::new("mixnode_id_counter");

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

pub const MIXNODE_REWARDING: Map<NodeId, MixNodeRewarding> =
    Map::new(MIXNODES_REWARDING_PK_NAMESPACE);

// TODO: should it exist here or in the rewards storage?
pub(crate) const HISTORICAL_PERIODS_RECORDS: Map<(NodeId, Period), HistoricalRewards> =
    Map::new(MIXNODES_HISTORICAL_RECORDS_PK_NAMESPACE);

fn dummy_get_current_epoch() -> EpochId {
    42
}

pub(crate) fn save_new_mixnode(
    storage: &mut dyn Storage,
    env: Env,
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    owner: Addr,
    proxy: Option<Addr>,
    pledge: Coin,
) -> Result<(NodeId, Layer), MixnetContractError> {
    let layer = assign_layer(storage)?;
    let node_id = next_mixnode_id_counter(storage)?;

    // TODO: to be replaced with proper call on the epoch storage
    let current_epoch = dummy_get_current_epoch();

    let mixnode_rewarding = MixNodeRewarding::initialise_new(cost_params, &pledge, current_epoch);
    let mixnode_bond = MixNodeBond::new(
        node_id,
        owner,
        pledge,
        layer,
        mixnode,
        proxy,
        env.block.height,
    );
    // TODO: see if the zeroth record is still required
    let initial_record = HistoricalRewards::new_zeroth();

    // save mixnode bond data
    // note that this implicitly checks for uniqueness on identity key, sphinx key and owner
    mixnode_bonds().save(storage, node_id, &mixnode_bond)?;

    // save rewarding data
    MIXNODE_REWARDING.save(storage, node_id, &mixnode_rewarding)?;

    // save initial historical records
    HISTORICAL_PERIODS_RECORDS.save(storage, (node_id, 0), &initial_record)?;

    Ok((node_id, layer))
}

pub(crate) fn cleanup_mixnode_storage(
    storage: &mut dyn Storage,
    current_details: &MixNodeDetails,
) -> Result<(), MixnetContractError> {
    let node_id = current_details.bond_information.id;
    // remove all bond information (we don't need it anymore
    // note that "normal" remove is `may_load` followed by `replace` with a `None`
    // and we have already loaded the data from the storage
    mixnode_bonds().replace(
        storage,
        node_id,
        None,
        Some(&current_details.bond_information),
    )?;

    // if there are no pending delegations to return, we can also
    // purge all information regarding rewarding parameters
    if current_details.rewarding_details.delegates == Decimal::zero() {
        MIXNODE_REWARDING.remove(storage, node_id);
    }

    // TODO: this depends whether we are actually creating this entry or not
    // HISTORICAL_PERIODS_RECORDS.remove(storage, (node_id, 0));

    decrement_layer_count(storage, current_details.bond_information.layer)
}

//
// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
// pub struct StoredMixnodeBond {
//     pub pledge_amount: Coin,
//     pub owner: Addr,
//     pub layer: Layer,
//     pub block_height: u64,
//     pub mix_node: MixNode,
//     pub proxy: Option<Addr>,
//     pub accumulated_rewards: Option<Uint128>,
//     pub epoch_rewards: Option<NodeEpochRewards>,
// }
//
// impl From<MixNodeBond> for StoredMixnodeBond {
//     fn from(mixnode_bond: MixNodeBond) -> StoredMixnodeBond {
//         StoredMixnodeBond {
//             pledge_amount: mixnode_bond.pledge_amount,
//             owner: mixnode_bond.owner,
//             layer: mixnode_bond.layer,
//             block_height: mixnode_bond.block_height,
//             mix_node: mixnode_bond.mix_node,
//             proxy: mixnode_bond.proxy,
//             accumulated_rewards: mixnode_bond.accumulated_rewards,
//             epoch_rewards: None,
//         }
//     }
// }
//
// impl StoredMixnodeBond {
//     #[allow(clippy::too_many_arguments)]
//     pub(crate) fn new(
//         pledge_amount: Coin,
//         owner: Addr,
//         layer: Layer,
//         block_height: u64,
//         mix_node: MixNode,
//         proxy: Option<Addr>,
//         accumulated_rewards: Option<Uint128>,
//         epoch_rewards: Option<NodeEpochRewards>,
//     ) -> Self {
//         StoredMixnodeBond {
//             pledge_amount,
//             owner,
//             layer,
//             block_height,
//             mix_node,
//             proxy,
//             accumulated_rewards,
//             epoch_rewards,
//         }
//     }
//
//     pub(crate) fn accumulated_rewards(&self) -> Uint128 {
//         self.accumulated_rewards.unwrap_or_else(Uint128::zero)
//     }
//
//     pub(crate) fn attach_delegation(self, total_delegation: Uint128) -> MixNodeBond {
//         MixNodeBond {
//             total_delegation: Coin {
//                 denom: self.pledge_amount.denom.clone(),
//                 amount: total_delegation,
//             },
//             pledge_amount: self.pledge_amount,
//             owner: self.owner,
//             layer: self.layer,
//             block_height: self.block_height,
//             mix_node: self.mix_node,
//             proxy: self.proxy,
//             accumulated_rewards: self.accumulated_rewards,
//         }
//     }
//
//     pub(crate) fn identity(&self) -> &String {
//         &self.mix_node.identity_key
//     }
//
//     pub(crate) fn pledge_amount(&self) -> Coin {
//         self.pledge_amount.clone()
//     }
//
//     pub fn profit_margin(&self) -> U128 {
//         U128::from_num(self.mix_node.profit_margin_percent) / U128::from_num(100)
//     }
// }
//
// impl Display for StoredMixnodeBond {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "amount: {}, owner: {}, identity: {}",
//             self.pledge_amount, self.owner, self.mix_node.identity_key
//         )
//     }
// }
//
// pub(crate) fn read_full_mixnode_bond(
//     storage: &dyn Storage,
//     mix_identity: IdentityKeyRef<'_>,
// ) -> StdResult<Option<MixNodeBond>> {
//     let stored_bond = mixnodes().may_load(storage, mix_identity)?;
//     match stored_bond {
//         None => Ok(None),
//         Some(stored_bond) => {
//             let total_delegation = TOTAL_DELEGATION.may_load(storage, mix_identity)?;
//             Ok(Some(MixNodeBond {
//                 pledge_amount: stored_bond.pledge_amount,
//                 total_delegation: Coin {
//                     denom: MIX_DENOM.base.to_owned(),
//                     amount: total_delegation.unwrap_or_default(),
//                 },
//                 owner: stored_bond.owner,
//                 layer: stored_bond.layer,
//                 block_height: stored_bond.block_height,
//                 mix_node: stored_bond.mix_node,
//                 proxy: stored_bond.proxy,
//                 accumulated_rewards: stored_bond.accumulated_rewards,
//             }))
//         }
//     }
// }
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
