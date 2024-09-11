// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use crate::mixnodes::storage::{assign_layer, next_mixnode_id_counter};
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Addr, Coin, Decimal, Env, StdResult, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::{
    MixNodeCostParams, MixNodeDetails, MixNodeRewarding, UnbondedMixnode,
};
use mixnet_contract_common::{IdentityKey, Layer, MixId, MixNode, MixNodeBond};

pub(crate) fn must_get_mixnode_bond_by_owner(
    store: &dyn Storage,
    owner: &Addr,
) -> Result<MixNodeBond, MixnetContractError> {
    Ok(storage::mixnode_bonds()
        .idx
        .owner
        .item(store, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        })?
        .1)
}

pub(crate) fn attach_mix_details(
    store: &dyn Storage,
    bond_information: MixNodeBond,
) -> StdResult<MixNodeDetails> {
    // if bond exists, rewarding details MUST also exist
    let rewarding_details =
        rewards_storage::MIXNODE_REWARDING.load(store, bond_information.mix_id)?;

    // since this `Map` hasn't existed when contract was instantiated, some mixnodes might not
    // have an entry here. But that's fine, because it means they have no pending changes
    // (if there were supposed to be any changes, they would have been added during migration)
    let pending_changes = storage::PENDING_MIXNODE_CHANGES
        .may_load(store, bond_information.mix_id)?
        .unwrap_or_default();

    Ok(MixNodeDetails::new(
        bond_information,
        rewarding_details,
        pending_changes,
    ))
}

pub(crate) fn get_mixnode_details_by_id(
    store: &dyn Storage,
    mix_id: MixId,
) -> StdResult<Option<MixNodeDetails>> {
    if let Some(bond_information) = storage::mixnode_bonds().may_load(store, mix_id)? {
        attach_mix_details(store, bond_information).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn get_mixnode_details_by_owner(
    store: &dyn Storage,
    address: Addr,
) -> StdResult<Option<MixNodeDetails>> {
    if let Some(bond_information) = storage::mixnode_bonds()
        .idx
        .owner
        .item(store, address)?
        .map(|record| record.1)
    {
        attach_mix_details(store, bond_information).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn get_mixnode_details_by_identity(
    store: &dyn Storage,
    mix_identity: IdentityKey,
) -> StdResult<Option<MixNodeDetails>> {
    if let Some(bond_information) = storage::mixnode_bonds()
        .idx
        .identity_key
        .item(store, mix_identity)?
        .map(|record| record.1)
    {
        attach_mix_details(store, bond_information).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn save_new_mixnode(
    storage: &mut dyn Storage,
    env: Env,
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    owner: Addr,
    pledge: Coin,
) -> Result<(MixId, Layer), MixnetContractError> {
    let layer = assign_layer(storage)?;
    let mix_id = next_mixnode_id_counter(storage)?;
    let current_epoch = interval_storage::current_interval(storage)?.current_epoch_absolute_id();

    let mixnode_rewarding = MixNodeRewarding::initialise_new(cost_params, &pledge, current_epoch)?;
    let mixnode_bond = MixNodeBond::new(mix_id, owner, pledge, layer, mixnode, env.block.height);

    // save mixnode bond data
    // note that this implicitly checks for uniqueness on identity key, sphinx key and owner
    storage::mixnode_bonds().save(storage, mix_id, &mixnode_bond)?;

    // save rewarding data
    rewards_storage::MIXNODE_REWARDING.save(storage, mix_id, &mixnode_rewarding)?;

    Ok((mix_id, layer))
}

pub(crate) fn cleanup_post_unbond_mixnode_storage(
    storage: &mut dyn Storage,
    env: &Env,
    current_details: &MixNodeDetails,
) -> Result<(), MixnetContractError> {
    let mix_id = current_details.bond_information.mix_id;
    // remove all bond information (we don't need it anymore
    // note that "normal" remove is `may_load` followed by `replace` with a `None`
    // and we have already loaded the data from the storage
    storage::mixnode_bonds().replace(
        storage,
        mix_id,
        None,
        Some(&current_details.bond_information),
    )?;

    // if there are no pending delegations to return, we can also
    // purge all information regarding rewarding parameters
    if current_details.rewarding_details.unique_delegations == 0 {
        rewards_storage::MIXNODE_REWARDING.remove(storage, mix_id);
    } else {
        // otherwise just set operator's tokens to zero as to indicate they have unbonded
        // and already claimed those
        let mut zeroed = current_details.rewarding_details.clone();
        zeroed.operator = Decimal::zero();

        rewards_storage::MIXNODE_REWARDING.save(storage, mix_id, &zeroed)?;
    }

    let identity = current_details.bond_information.identity().to_owned();
    let owner = current_details.bond_information.owner().to_owned();
    let proxy = current_details.bond_information.proxy.to_owned();

    // save minimal information about this mixnode
    storage::unbonded_mixnodes().save(
        storage,
        mix_id,
        &UnbondedMixnode {
            identity_key: identity,
            owner,
            proxy,
            unbonding_height: env.block.height,
        },
    )?;
    storage::decrement_layer_count(storage, current_details.bond_information.layer)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::fixtures::{
        mix_node_cost_params_fixture, mix_node_fixture, TEST_COIN_DENOM,
    };
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::{coin, Uint128};

    pub(crate) struct DummyMixnode {
        pub mix_id: MixId,
        pub owner: Addr,
        pub identity: IdentityKey,
    }

    pub(crate) const OWNER_EXISTS: &str = "mix-owner-existing";
    pub(crate) const OWNER_UNBONDING: &str = "mix-owner-unbonding";
    pub(crate) const OWNER_UNBONDED: &str = "mix-owner-unbonded";
    pub(crate) const OWNER_UNBONDED_LEFTOVER: &str = "mix-owner-unbonded-leftover";

    // create a mixnode that is bonded, unbonded, in the process of unbonding and unbonded with leftover mix rewarding details
    pub(crate) fn setup_mix_combinations(
        test: &mut TestSetup,
        stake: Option<Uint128>,
    ) -> Vec<DummyMixnode> {
        let (mix_id, keypair) = test.add_dummy_mixnode_with_keypair(OWNER_EXISTS, stake);
        let mix_exists = DummyMixnode {
            mix_id,
            owner: Addr::unchecked(OWNER_EXISTS),
            identity: keypair.public_key().to_base58_string(),
        };

        let (mix_id, keypair) = test.add_dummy_mixnode_with_keypair(OWNER_UNBONDING, stake);
        let mix_unbonding = DummyMixnode {
            mix_id,
            owner: Addr::unchecked(OWNER_UNBONDING),
            identity: keypair.public_key().to_base58_string(),
        };

        let (mix_id, keypair) = test.add_dummy_mixnode_with_keypair(OWNER_UNBONDED, stake);
        let mix_unbonded = DummyMixnode {
            mix_id,
            owner: Addr::unchecked(OWNER_UNBONDED),
            identity: keypair.public_key().to_base58_string(),
        };

        let (mix_id, keypair) = test.add_dummy_mixnode_with_keypair(OWNER_UNBONDED_LEFTOVER, stake);
        let mix_unbonded_leftover = DummyMixnode {
            mix_id,
            owner: Addr::unchecked(OWNER_UNBONDED_LEFTOVER),
            identity: keypair.public_key().to_base58_string(),
        };

        // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
        let mut rewarding_details = test.mix_rewarding(mix_unbonded_leftover.mix_id);
        rewarding_details.delegates = Decimal::raw(12345);
        rewarding_details.unique_delegations = 1;
        rewards_storage::MIXNODE_REWARDING
            .save(
                test.deps_mut().storage,
                mix_unbonded_leftover.mix_id,
                &rewarding_details,
            )
            .unwrap();

        test.immediately_unbond_mixnode(mix_unbonded.mix_id);
        test.immediately_unbond_mixnode(mix_unbonded_leftover.mix_id);
        test.start_unbonding_mixnode(mix_unbonding.mix_id);

        vec![
            mix_exists,
            mix_unbonding,
            mix_unbonded,
            mix_unbonded_leftover,
        ]
    }

    #[test]
    fn getting_mixnode_bond_by_owner() {
        let mut test = TestSetup::new();

        let nodes = setup_mix_combinations(&mut test, None);
        let mix_exists = &nodes[0];
        let mix_unbonding = &nodes[1];
        let mix_unbonded = &nodes[2];
        let mix_unbonded_leftover = &nodes[3];

        // if this is a normally bonded mixnode, all should be fine
        let res = must_get_mixnode_bond_by_owner(test.deps().storage, &mix_exists.owner).unwrap();
        assert_eq!(res.mix_id, mix_exists.mix_id);

        // if node is in the process of unbonding, we still should be capable of retrieving its details
        let res =
            must_get_mixnode_bond_by_owner(test.deps().storage, &mix_unbonding.owner).unwrap();
        assert_eq!(res.mix_id, mix_unbonding.mix_id);

        // but if node has unbonded, the information is purged and query fails
        let res = must_get_mixnode_bond_by_owner(test.deps().storage, &mix_unbonded.owner);
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: mix_unbonded.owner.clone()
            })
        );

        let res = must_get_mixnode_bond_by_owner(test.deps().storage, &mix_unbonded_leftover.owner);
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: mix_unbonded_leftover.owner.clone()
            })
        );
    }

    #[test]
    fn getting_mixnode_details_by_id() {
        let mut test = TestSetup::new();

        let nodes = setup_mix_combinations(&mut test, None);
        let mix_exists = &nodes[0];
        let mix_unbonding = &nodes[1];
        let mix_unbonded = &nodes[2];
        let mix_unbonded_leftover = &nodes[3];

        let res = get_mixnode_details_by_id(test.deps().storage, mix_exists.mix_id)
            .unwrap()
            .unwrap();
        assert_eq!(res.bond_information.mix_id, mix_exists.mix_id);

        let res = get_mixnode_details_by_id(test.deps().storage, mix_unbonding.mix_id)
            .unwrap()
            .unwrap();
        assert_eq!(res.bond_information.mix_id, mix_unbonding.mix_id);

        let res = get_mixnode_details_by_id(test.deps().storage, mix_unbonded.mix_id).unwrap();
        assert!(res.is_none());

        let res =
            get_mixnode_details_by_id(test.deps().storage, mix_unbonded_leftover.mix_id).unwrap();
        assert!(res.is_none())
    }

    #[test]
    fn getting_mixnode_details_by_owner() {
        let mut test = TestSetup::new();

        let nodes = setup_mix_combinations(&mut test, None);
        let mix_exists = &nodes[0];
        let mix_unbonding = &nodes[1];
        let mix_unbonded = &nodes[2];
        let mix_unbonded_leftover = &nodes[3];

        // if this is a normally bonded mixnode, all should be fine
        let res = get_mixnode_details_by_owner(test.deps().storage, mix_exists.owner.clone())
            .unwrap()
            .unwrap();
        assert_eq!(res.bond_information.mix_id, mix_exists.mix_id);

        // if node is in the process of unbonding, we still should be capable of retrieving its details
        let res = get_mixnode_details_by_owner(test.deps().storage, mix_unbonding.owner.clone())
            .unwrap()
            .unwrap();
        assert_eq!(res.bond_information.mix_id, mix_unbonding.mix_id);

        // but if node has unbonded, the information is purged and query fails
        let res =
            get_mixnode_details_by_owner(test.deps().storage, mix_unbonded.owner.clone()).unwrap();
        assert!(res.is_none());

        let res =
            get_mixnode_details_by_owner(test.deps().storage, mix_unbonded_leftover.owner.clone())
                .unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn getting_mixnode_details_by_identity() {
        let mut test = TestSetup::new();

        let nodes = setup_mix_combinations(&mut test, None);
        let mix_exists = &nodes[0];
        let mix_unbonding = &nodes[1];
        let mix_unbonded = &nodes[2];
        let mix_unbonded_leftover = &nodes[3];

        // if this is a normally bonded mixnode, all should be fine
        let res = get_mixnode_details_by_identity(test.deps().storage, mix_exists.identity.clone())
            .unwrap()
            .unwrap();
        assert_eq!(res.bond_information.mix_id, mix_exists.mix_id);

        // if node is in the process of unbonding, we still should be capable of retrieving its details
        let res =
            get_mixnode_details_by_identity(test.deps().storage, mix_unbonding.identity.clone())
                .unwrap()
                .unwrap();
        assert_eq!(res.bond_information.mix_id, mix_unbonding.mix_id);

        // but if node has unbonded, the information is purged and query fails
        let res =
            get_mixnode_details_by_identity(test.deps().storage, mix_unbonded.identity.clone())
                .unwrap();
        assert!(res.is_none());

        let res = get_mixnode_details_by_identity(
            test.deps().storage,
            mix_unbonded_leftover.identity.clone(),
        )
        .unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn saving_new_mixnode() {
        let mut test = TestSetup::new();

        // get some mixnodes in
        test.add_dummy_mixnode("owner1", None);
        test.add_dummy_mixnode("owner2", None);
        test.add_dummy_mixnode("owner3", None);
        test.add_dummy_mixnode("owner4", None);
        test.add_dummy_mixnode("owner5", None);

        let env = test.env();
        let id_key = "identity-key";
        let sphinx_key = "sphinx-key";
        let mut mixnode = mix_node_fixture();
        mixnode.identity_key = id_key.into();
        mixnode.sphinx_key = sphinx_key.into();
        let cost_params = mix_node_cost_params_fixture();
        let owner = Addr::unchecked("mix-owner");
        let pledge = coin(100_000_000, TEST_COIN_DENOM);

        let (id, layer) = save_new_mixnode(
            test.deps_mut().storage,
            env.clone(),
            mixnode,
            cost_params.clone(),
            owner.clone(),
            pledge.clone(),
        )
        .unwrap();
        assert_eq!(id, 6);
        assert_eq!(layer, Layer::Three);

        assert_eq!(
            storage::MIXNODE_ID_COUNTER
                .load(test.deps().storage)
                .unwrap(),
            6
        );
        assert_eq!(storage::LAYERS.load(test.deps().storage).unwrap().layer3, 2);
        let mix_details = get_mixnode_details_by_id(test.deps().storage, id)
            .unwrap()
            .unwrap();
        assert_eq!(mix_details.mix_id(), id);
        assert_eq!(mix_details.original_pledge(), &pledge);
        assert_eq!(
            mix_details.bond_information.bonding_height,
            env.block.height
        );

        // try to add node with duplicate identity...
        let mut mixnode = mix_node_fixture();
        mixnode.identity_key = id_key.into();
        let res = save_new_mixnode(
            test.deps_mut().storage,
            env.clone(),
            mixnode,
            cost_params.clone(),
            Addr::unchecked("different-owner"),
            pledge.clone(),
        );
        assert!(res.is_err());

        // and duplicate owner...
        let mixnode = mix_node_fixture();
        let res = save_new_mixnode(
            test.deps_mut().storage,
            env.clone(),
            mixnode,
            cost_params.clone(),
            owner,
            pledge.clone(),
        );
        assert!(res.is_err());

        // and duplicate sphinx key...
        let mut mixnode = mix_node_fixture();
        mixnode.sphinx_key = sphinx_key.into();
        let res = save_new_mixnode(
            test.deps_mut().storage,
            env,
            mixnode,
            cost_params,
            Addr::unchecked("different-owner"),
            pledge,
        );
        assert!(res.is_err());
    }

    #[test]
    fn cleaning_post_unbond_storage() {
        let mut test = TestSetup::new();

        let mix_id = test.add_dummy_mixnode("mix-owner", None);
        let mix_id_leftover = test.add_dummy_mixnode("mix-owner-leftover", None);

        // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
        let mut rewarding_details = test.mix_rewarding(mix_id_leftover);
        rewarding_details.delegates = Decimal::raw(12345);
        rewarding_details.unique_delegations = 1;
        rewards_storage::MIXNODE_REWARDING
            .save(test.deps_mut().storage, mix_id_leftover, &rewarding_details)
            .unwrap();

        let bond1 = storage::mixnode_bonds()
            .load(test.deps().storage, mix_id)
            .unwrap();
        let bond2 = storage::mixnode_bonds()
            .load(test.deps().storage, mix_id_leftover)
            .unwrap();

        let env = test.env();
        let details1 = get_mixnode_details_by_id(test.deps().storage, mix_id)
            .unwrap()
            .unwrap();
        cleanup_post_unbond_mixnode_storage(test.deps_mut().storage, &env, &details1).unwrap();

        // bond information is gone
        let bond = storage::mixnode_bonds()
            .may_load(test.deps().storage, mix_id)
            .unwrap();
        assert!(bond.is_none());

        // rewarding information is gone
        let mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .may_load(test.deps().storage, mix_id)
            .unwrap();
        assert!(mix_rewarding.is_none());

        // unbonded details are inserted
        let unbonded_details = storage::unbonded_mixnodes()
            .load(test.deps().storage, mix_id)
            .unwrap();
        let expected = UnbondedMixnode {
            identity_key: bond1.mix_node.identity_key,
            owner: bond1.owner,
            proxy: None,
            unbonding_height: env.block.height,
        };
        assert_eq!(unbonded_details, expected);

        // layers are decremented
        assert_eq!(storage::LAYERS.load(test.deps().storage).unwrap().layer1, 0);

        let details2 = get_mixnode_details_by_id(test.deps().storage, mix_id_leftover)
            .unwrap()
            .unwrap();
        cleanup_post_unbond_mixnode_storage(test.deps_mut().storage, &env, &details2).unwrap();

        // bond information is gone
        let bond = storage::mixnode_bonds()
            .may_load(test.deps().storage, mix_id_leftover)
            .unwrap();
        assert!(bond.is_none());

        // rewarding information is NOT gone, but operator field is reset
        let mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .may_load(test.deps().storage, mix_id_leftover)
            .unwrap()
            .unwrap();
        assert!(mix_rewarding.operator.is_zero());
        assert_eq!(mix_rewarding.unique_delegations, 1);

        // unbonded details are inserted
        let unbonded_details = storage::unbonded_mixnodes()
            .load(test.deps().storage, mix_id_leftover)
            .unwrap();
        let expected = UnbondedMixnode {
            identity_key: bond2.mix_node.identity_key,
            owner: bond2.owner,
            proxy: None,
            unbonding_height: env.block.height,
        };
        assert_eq!(unbonded_details, expected);

        // layers are decremented
        assert_eq!(storage::LAYERS.load(test.deps().storage).unwrap().layer2, 0);
    }
}
