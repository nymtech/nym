// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{Addr, Deps};
use node_families_contract_common::NodeFamiliesContractError;
use nym_mixnet_contract_common::{MixnetContractQuerier, NodeId};

/// Normalise a family name into the canonical form used as the unique-index key.
///
/// Drops every character that isn't an ASCII letter or digit and lowercases
/// the rest, so `"  Foo-Bar! "`, `"foobar"` and `"FOO BAR"` all collide on
/// the storage layer's unique-name index.
pub fn normalise_family_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Ensure no node controlled by `address` is currently a member of any family.
pub(crate) fn ensure_address_holds_no_family_membership(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    address: &Addr,
) -> Result<(), NodeFamiliesContractError> {
    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    let Some(nym_node) = deps
        .querier
        .query_nymnode_ownership(&mixnet_contract, address)?
    else {
        // if the owner has no nym-node, it can't possibly be in a family
        return Ok(());
    };

    // check if that node is in a family
    if let Some(family) = storage
        .family_members
        .may_load(deps.storage, nym_node.node_id)?
    {
        return Err(NodeFamiliesContractError::AlreadyInFamily {
            address: address.clone(),
            node_id: nym_node.node_id,
            family_id: family.family_id,
        });
    }

    Ok(())
}

/// Cross-contract query: ensure `node_id` is a currently-bonded node in the
/// mixnet contract. Returns [`NodeDoesntExist`] otherwise.
///
/// [`NodeDoesntExist`]: NodeFamiliesContractError::NodeDoesntExist
pub(crate) fn ensure_node_is_bonded(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    node_id: NodeId,
) -> Result<(), NodeFamiliesContractError> {
    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    if !deps
        .querier
        .check_node_existence(&mixnet_contract, node_id)?
    {
        return Err(NodeFamiliesContractError::NodeDoesntExist { node_id });
    }
    Ok(())
}

/// Ensure `node_id` is not currently a member of any family. Returns
/// [`NodeAlreadyInFamily`] if it is.
///
/// [`NodeAlreadyInFamily`]: NodeFamiliesContractError::NodeAlreadyInFamily
pub(crate) fn ensure_node_not_in_family(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    node_id: NodeId,
) -> Result<(), NodeFamiliesContractError> {
    if let Some(membership) = storage.family_members.may_load(deps.storage, node_id)? {
        return Err(NodeFamiliesContractError::NodeAlreadyInFamily {
            node_id,
            family_id: membership.family_id,
        });
    }
    Ok(())
}
