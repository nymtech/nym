// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage as interval_storage;
use crate::nodes::storage;
use crate::nodes::storage::next_nymnode_id_counter;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Addr, Coin, Env, StdResult, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::UnbondedNymNode;
use mixnet_contract_common::{
    NodeCostParams, NodeId, NodeRewarding, NymNode, NymNodeBond, NymNodeDetails, PendingNodeChanges,
};
use nym_contracts_common::IdentityKey;

pub(crate) fn save_new_nymnode(
    storage: &mut dyn Storage,
    bonding_height: u64,
    node: NymNode,
    cost_params: NodeCostParams,
    owner: Addr,
    pledge: Coin,
) -> Result<NodeId, MixnetContractError> {
    let node_id = next_nymnode_id_counter(storage)?;
    save_new_nymnode_with_id(
        storage,
        node_id,
        bonding_height,
        node,
        cost_params,
        owner,
        pledge,
    )?;

    Ok(node_id)
}

pub(crate) fn save_new_nymnode_with_id(
    storage: &mut dyn Storage,
    node_id: NodeId,
    bonding_height: u64,
    node: NymNode,
    cost_params: NodeCostParams,
    owner: Addr,
    pledge: Coin,
) -> Result<(), MixnetContractError> {
    let current_epoch = interval_storage::current_interval(storage)?.current_epoch_absolute_id();

    let node_rewarding = NodeRewarding::initialise_new(cost_params, &pledge, current_epoch)?;
    let node_bond = NymNodeBond::new(node_id, owner, pledge, node, bonding_height);

    // save node bond data
    // note that this implicitly checks for uniqueness on identity key and owner
    storage::nym_nodes().save(storage, node_id, &node_bond)?;

    // save rewarding data
    rewards_storage::NYMNODE_REWARDING.save(storage, node_id, &node_rewarding)?;

    // initialise pending changes
    storage::PENDING_NYMNODE_CHANGES.save(storage, node_id, &PendingNodeChanges::new_empty())?;

    Ok(())
}

pub(crate) fn attach_nym_node_details(
    store: &dyn Storage,
    bond_information: NymNodeBond,
) -> StdResult<NymNodeDetails> {
    // if bond exists, rewarding details MUST also exist
    let rewarding_details =
        rewards_storage::NYMNODE_REWARDING.load(store, bond_information.node_id)?;

    // the same is true for the pending changes
    let pending_changes = storage::PENDING_NYMNODE_CHANGES.load(store, bond_information.node_id)?;

    Ok(NymNodeDetails::new(
        bond_information,
        rewarding_details,
        pending_changes,
    ))
}

pub(crate) fn get_node_details_by_id(
    store: &dyn Storage,
    mix_id: NodeId,
) -> StdResult<Option<NymNodeDetails>> {
    if let Some(bond_information) = storage::nym_nodes().may_load(store, mix_id)? {
        attach_nym_node_details(store, bond_information).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn get_node_details_by_owner(
    store: &dyn Storage,
    address: Addr,
) -> StdResult<Option<NymNodeDetails>> {
    if let Some(bond_information) = storage::nym_nodes()
        .idx
        .owner
        .item(store, address)?
        .map(|record| record.1)
    {
        attach_nym_node_details(store, bond_information).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn get_node_details_by_identity(
    store: &dyn Storage,
    identity: IdentityKey,
) -> StdResult<Option<NymNodeDetails>> {
    if let Some(bond_information) = storage::nym_nodes()
        .idx
        .identity_key
        .item(store, identity)?
        .map(|record| record.1)
    {
        attach_nym_node_details(store, bond_information).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn must_get_node_bond_by_owner(
    store: &dyn Storage,
    owner: &Addr,
) -> Result<NymNodeBond, MixnetContractError> {
    Ok(storage::nym_nodes()
        .idx
        .owner
        .item(store, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedNodeBond {
            owner: owner.clone(),
        })?
        .1)
}

pub(crate) fn cleanup_post_unbond_nym_node_storage(
    storage: &mut dyn Storage,
    env: &Env,
    current_details: &NymNodeDetails,
) -> Result<(), MixnetContractError> {
    let node_id = current_details.bond_information.node_id;
    // remove all bond information since we don't need it anymore
    // note that "normal" remove is `may_load` followed by `replace` with a `None`
    // and we have already loaded the data from the storage
    storage::nym_nodes().replace(
        storage,
        node_id,
        None,
        Some(&current_details.bond_information),
    )?;

    // if there are no pending delegations to return, we can also
    // purge all information regarding rewarding parameters
    if current_details.rewarding_details.unique_delegations == 0 {
        rewards_storage::NYMNODE_REWARDING.remove(storage, node_id);
    } else {
        // otherwise just set operator's tokens to zero as to indicate they have unbonded
        // and already claimed those
        let zeroed = current_details.rewarding_details.clear_operator();
        rewards_storage::NYMNODE_REWARDING.save(storage, node_id, &zeroed)?;
    }

    let identity_key = current_details.bond_information.identity().to_owned();
    let owner = current_details.bond_information.owner.clone();

    // save minimal information about this node
    storage::unbonded_nym_nodes().save(
        storage,
        node_id,
        &UnbondedNymNode {
            identity_key,
            node_id,
            owner,
            unbonding_height: env.block.height,
        },
    )?;

    Ok(())
}
