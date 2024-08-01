// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{NYM_NODE_BOND_DEFAULT_RETRIEVAL_LIMIT, NYM_NODE_BOND_MAX_RETRIEVAL_LIMIT};
use crate::nodes::storage;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::{
    EpochAssignmentResponse, PagedNymNodesResponse, Role, RolesMetadataResponse,
};
use mixnet_contract_common::NodeId;

pub(crate) fn query_nymnodes_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedNymNodesResponse> {
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

    Ok(PagedNymNodesResponse::new(nodes, limit, start_next_after))
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
