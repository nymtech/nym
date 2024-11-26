// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    NYM_NODE_BOND_DEFAULT_RETRIEVAL_LIMIT, NYM_NODE_BOND_MAX_RETRIEVAL_LIMIT,
    NYM_NODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT, NYM_NODE_DETAILS_MAX_RETRIEVAL_LIMIT,
    UNBONDED_NYM_NODES_DEFAULT_RETRIEVAL_LIMIT, UNBONDED_NYM_NODES_MAX_RETRIEVAL_LIMIT,
};
use crate::nodes::helpers::{
    attach_nym_node_details, get_node_details_by_id, get_node_details_by_identity,
    get_node_details_by_owner,
};
use crate::nodes::storage;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Deps, Order, StdResult, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::{
    EpochAssignmentResponse, NodeDetailsByIdentityResponse, NodeDetailsResponse,
    NodeOwnershipResponse, NodeRewardingDetailsResponse, PagedNymNodeBondsResponse,
    PagedNymNodeDetailsResponse, PagedUnbondedNymNodesResponse, Role, RolesMetadataResponse,
    StakeSaturationResponse, UnbondedNodeResponse,
};
use mixnet_contract_common::{NodeId, NymNodeBond, NymNodeDetails};
use nym_contracts_common::IdentityKey;

pub(crate) fn query_nymnode_bonds_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedNymNodeBondsResponse> {
    let limit = limit
        .unwrap_or(NYM_NODE_BOND_DEFAULT_RETRIEVAL_LIMIT)
        .min(NYM_NODE_BOND_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::nym_nodes()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|node| node.node_id);

    Ok(PagedNymNodeBondsResponse {
        nodes,
        start_next_after,
    })
}

pub(crate) fn query_rewarded_set_metadata(
    deps: Deps<'_>,
) -> Result<RolesMetadataResponse, MixnetContractError> {
    let metadata = storage::read_rewarded_set_metadata(deps.storage)?;
    Ok(RolesMetadataResponse { metadata })
}

pub(crate) fn query_epoch_assignment(
    deps: Deps<'_>,
    role: Role,
) -> Result<EpochAssignmentResponse, MixnetContractError> {
    let metadata = storage::read_rewarded_set_metadata(deps.storage)?;
    let nodes = storage::read_assigned_roles(deps.storage, role)?;
    Ok(EpochAssignmentResponse {
        epoch_id: metadata.epoch_id,
        nodes,
    })
}

fn attach_node_details(
    storage: &dyn Storage,
    read_bond: StdResult<(NodeId, NymNodeBond)>,
) -> StdResult<NymNodeDetails> {
    match read_bond {
        Ok((_, bond)) => attach_nym_node_details(storage, bond),
        Err(err) => Err(err),
    }
}

pub fn query_nymnodes_details_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedNymNodeDetailsResponse> {
    let limit = limit
        .unwrap_or(NYM_NODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT)
        .min(NYM_NODE_DETAILS_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::nym_nodes()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| attach_node_details(deps.storage, res))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|details| details.node_id());

    Ok(PagedNymNodeDetailsResponse {
        nodes,
        start_next_after,
    })
}

pub fn query_unbonded_nymnodes_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedUnbondedNymNodesResponse> {
    let limit = limit
        .unwrap_or(UNBONDED_NYM_NODES_DEFAULT_RETRIEVAL_LIMIT)
        .min(UNBONDED_NYM_NODES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::unbonded_nym_nodes()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|res| res.node_id);

    Ok(PagedUnbondedNymNodesResponse {
        nodes,
        start_next_after,
    })
}

pub fn query_unbonded_nymnodes_by_owner_paged(
    deps: Deps<'_>,
    owner: String,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedUnbondedNymNodesResponse> {
    let owner = deps.api.addr_validate(&owner)?;

    let limit = limit
        .unwrap_or(UNBONDED_NYM_NODES_DEFAULT_RETRIEVAL_LIMIT)
        .min(UNBONDED_NYM_NODES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::unbonded_nym_nodes()
        .idx
        .owner
        .prefix(owner)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|r| r.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|res| res.node_id);

    Ok(PagedUnbondedNymNodesResponse {
        nodes,
        start_next_after,
    })
}

pub fn query_unbonded_nymnodes_by_identity_paged(
    deps: Deps<'_>,
    identity_key: String,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedUnbondedNymNodesResponse> {
    let limit = limit
        .unwrap_or(UNBONDED_NYM_NODES_DEFAULT_RETRIEVAL_LIMIT)
        .min(UNBONDED_NYM_NODES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::unbonded_nym_nodes()
        .idx
        .identity_key
        .prefix(identity_key)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|r| r.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|res| res.node_id);

    Ok(PagedUnbondedNymNodesResponse {
        nodes,
        start_next_after,
    })
}

pub fn query_owned_nymnode(deps: Deps<'_>, address: String) -> StdResult<NodeOwnershipResponse> {
    let validated_addr = deps.api.addr_validate(&address)?;

    let details = get_node_details_by_owner(deps.storage, validated_addr.clone())?;
    Ok(NodeOwnershipResponse {
        address: validated_addr,
        details,
    })
}

pub fn query_nymnode_details(deps: Deps<'_>, node_id: NodeId) -> StdResult<NodeDetailsResponse> {
    let details = get_node_details_by_id(deps.storage, node_id)?;

    Ok(NodeDetailsResponse { node_id, details })
}

pub fn query_nymnode_details_by_identity(
    deps: Deps<'_>,
    identity_key: IdentityKey,
) -> StdResult<NodeDetailsByIdentityResponse> {
    let details = get_node_details_by_identity(deps.storage, identity_key.clone())?;

    Ok(NodeDetailsByIdentityResponse {
        identity_key,
        details,
    })
}

pub fn query_nymnode_rewarding_details(
    deps: Deps<'_>,
    node_id: NodeId,
) -> StdResult<NodeRewardingDetailsResponse> {
    let rewarding_details = rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, node_id)?;

    Ok(NodeRewardingDetailsResponse {
        node_id,
        rewarding_details,
    })
}

pub fn query_unbonded_nymnode(deps: Deps<'_>, node_id: NodeId) -> StdResult<UnbondedNodeResponse> {
    let details = storage::unbonded_nym_nodes().may_load(deps.storage, node_id)?;

    Ok(UnbondedNodeResponse { node_id, details })
}

pub fn query_stake_saturation(
    deps: Deps<'_>,
    node_id: NodeId,
) -> StdResult<StakeSaturationResponse> {
    let node_rewarding = match rewards_storage::NYMNODE_REWARDING.may_load(deps.storage, node_id)? {
        Some(node_rewarding) => node_rewarding,
        None => {
            return Ok(StakeSaturationResponse {
                node_id,
                current_saturation: None,
                uncapped_saturation: None,
            })
        }
    };

    let rewarding_params = rewards_storage::REWARDING_PARAMS.load(deps.storage)?;

    Ok(StakeSaturationResponse {
        node_id,
        current_saturation: Some(node_rewarding.bond_saturation(&rewarding_params)),
        uncapped_saturation: Some(node_rewarding.uncapped_bond_saturation(&rewarding_params)),
    })
}
