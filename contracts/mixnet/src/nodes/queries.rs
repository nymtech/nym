// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nodes::storage;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::{EpochAssignmentResponse, Role, RolesMetadataResponse};
use mixnet_contract_common::NodeId;

pub(crate) fn query_nymnodes_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) {
    todo!()
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
