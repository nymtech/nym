// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
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

// keeps track of `node_id -> IdentityKey, Owner, unbonding_height` so we'd known a bit more about past mixnodes
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

pub(crate) fn initialise_storage(storage: &mut dyn Storage) -> StdResult<()> {
    LAYERS.save(storage, &LayerDistribution::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn decrementing_layer() {
        let mut deps = test_helpers::init_contract();

        // we never underflow, if it were to happen we're going to return an error instead
        assert_eq!(
            Err(MixnetContractError::OverflowSubtraction {
                minuend: 0,
                subtrahend: 1
            }),
            decrement_layer_count(deps.as_mut().storage, Layer::One)
        );

        LAYERS
            .save(
                deps.as_mut().storage,
                &LayerDistribution {
                    layer1: 3,
                    layer2: 2,
                    layer3: 1,
                },
            )
            .unwrap();

        assert!(decrement_layer_count(deps.as_mut().storage, Layer::One).is_ok());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Two).is_ok());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Three).is_ok());

        assert!(decrement_layer_count(deps.as_mut().storage, Layer::One).is_ok());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Two).is_ok());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Three).is_err());

        assert!(decrement_layer_count(deps.as_mut().storage, Layer::One).is_ok());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Two).is_err());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Three).is_err());

        assert!(decrement_layer_count(deps.as_mut().storage, Layer::One).is_err());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Two).is_err());
        assert!(decrement_layer_count(deps.as_mut().storage, Layer::Three).is_err());
    }

    #[test]
    fn assigning_layer() {
        let mut deps = test_helpers::init_contract();

        let layers = LayerDistribution {
            layer1: 3,
            layer2: 2,
            layer3: 1,
        };
        LAYERS.save(deps.as_mut().storage, &layers).unwrap();

        // always assigns layer with fewest nodes
        assert_eq!(Layer::Three, assign_layer(deps.as_mut().storage).unwrap());
        assert_eq!(2, LAYERS.load(deps.as_ref().storage).unwrap().layer3);

        // we have 3, 2, 2, so the 2nd layer should get chosen now
        assert_eq!(Layer::Two, assign_layer(deps.as_mut().storage).unwrap());
        assert_eq!(3, LAYERS.load(deps.as_ref().storage).unwrap().layer2);

        // 3, 3, 2, so 3rd one again
        assert_eq!(Layer::Three, assign_layer(deps.as_mut().storage).unwrap());
        assert_eq!(3, LAYERS.load(deps.as_ref().storage).unwrap().layer3);
    }

    #[test]
    fn next_id() {
        let mut deps = test_helpers::init_contract();

        for i in 1u64..1000 {
            assert_eq!(i, next_mixnode_id_counter(deps.as_mut().storage).unwrap());
        }
    }

    #[test]
    fn initialising() {
        let mut deps = mock_dependencies();
        assert!(LAYERS.load(deps.as_ref().storage).is_err());

        initialise_storage(deps.as_mut().storage).unwrap();
        assert_eq!(
            LayerDistribution::default(),
            LAYERS.load(deps.as_ref().storage).unwrap()
        );
    }
}
