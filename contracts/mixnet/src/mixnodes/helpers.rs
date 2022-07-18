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
use mixnet_contract_common::{Layer, MixNode, MixNodeBond, NodeId};

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

pub(crate) fn get_mixnode_details_by_id(
    store: &dyn Storage,
    node_id: NodeId,
) -> StdResult<Option<MixNodeDetails>> {
    if let Some(bond_information) = storage::mixnode_bonds().may_load(store, node_id)? {
        // if bond exists, rewarding details MUST also exist
        let rewarding_details =
            rewards_storage::MIXNODE_REWARDING.load(store, bond_information.id)?;
        Ok(Some(MixNodeDetails::new(
            bond_information,
            rewarding_details,
        )))
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
        // if bond exists, rewarding details MUST also exist
        let rewarding_details =
            rewards_storage::MIXNODE_REWARDING.load(store, bond_information.id)?;
        Ok(Some(MixNodeDetails::new(
            bond_information,
            rewarding_details,
        )))
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
    proxy: Option<Addr>,
    pledge: Coin,
) -> Result<(NodeId, Layer), MixnetContractError> {
    let layer = assign_layer(storage)?;
    let node_id = next_mixnode_id_counter(storage)?;
    let current_epoch = interval_storage::current_interval(storage)?.current_full_epoch_id();

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

    // save mixnode bond data
    // note that this implicitly checks for uniqueness on identity key, sphinx key and owner
    storage::mixnode_bonds().save(storage, node_id, &mixnode_bond)?;

    // save rewarding data
    rewards_storage::MIXNODE_REWARDING.save(storage, node_id, &mixnode_rewarding)?;

    Ok((node_id, layer))
}

pub(crate) fn cleanup_post_unbond_mixnode_storage(
    storage: &mut dyn Storage,
    env: &Env,
    current_details: &MixNodeDetails,
) -> Result<(), MixnetContractError> {
    let node_id = current_details.bond_information.id;
    // remove all bond information (we don't need it anymore
    // note that "normal" remove is `may_load` followed by `replace` with a `None`
    // and we have already loaded the data from the storage
    storage::mixnode_bonds().replace(
        storage,
        node_id,
        None,
        Some(&current_details.bond_information),
    )?;

    // if there are no pending delegations to return, we can also
    // purge all information regarding rewarding parameters
    if current_details.rewarding_details.delegates == Decimal::zero() {
        rewards_storage::MIXNODE_REWARDING.remove(storage, node_id);
    } else {
        // otherwise just set operator's tokens to zero as to indicate they have unbonded
        // and already claimed those
        let mut zeroed = current_details.rewarding_details.clone();
        zeroed.operator = Decimal::zero();

        rewards_storage::MIXNODE_REWARDING.save(storage, node_id, &zeroed)?;
    }

    let identity = current_details.bond_information.identity().to_owned();
    let owner = current_details.bond_information.owner().to_owned();
    // save minimal information about this mixnode
    storage::UNBONDED_MIXNODES.save(
        storage,
        node_id,
        &UnbondedMixnode {
            identity,
            owner,
            unbonding_height: env.block.height,
        },
    )?;
    storage::decrement_layer_count(storage, current_details.bond_information.layer)
}
