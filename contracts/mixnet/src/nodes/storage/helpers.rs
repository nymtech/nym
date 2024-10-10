// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nodes::storage::rewarded_set::{ACTIVE_ROLES_BUCKET, ROLES, ROLES_METADATA};
use crate::nodes::storage::{nym_nodes, NYMNODE_ID_COUNTER};
use cosmwasm_std::{StdResult, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::{RewardedSetMetadata, Role};
use mixnet_contract_common::{EpochId, NodeId, NymNodeBond, RoleAssignment};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[repr(u8)]
pub enum RoleStorageBucket {
    #[default]
    A = 0,
    B = 1,
}

impl RoleStorageBucket {
    pub fn other(&self) -> Self {
        match self {
            RoleStorageBucket::A => RoleStorageBucket::B,
            RoleStorageBucket::B => RoleStorageBucket::A,
        }
    }

    pub fn swap(&self) -> Self {
        self.other()
    }
}

pub(crate) fn reset_inactive_metadata(
    storage: &mut dyn Storage,
    epoch_id: EpochId,
) -> Result<(), MixnetContractError> {
    let active_bucket = ACTIVE_ROLES_BUCKET.load(storage)?;
    let inactive = active_bucket.other() as u8;

    ROLES_METADATA.save(storage, inactive, &RewardedSetMetadata::new(epoch_id))?;
    Ok(())
}

pub(crate) fn save_assignment(
    storage: &mut dyn Storage,
    assignment: RoleAssignment,
) -> Result<(), MixnetContractError> {
    let active_bucket = ACTIVE_ROLES_BUCKET.load(storage)?;

    // we're always assigning to the INACTIVE bucket, because it's still being built
    let inactive = active_bucket.other() as u8;

    // update metadata
    let mut metadata = ROLES_METADATA.load(storage, inactive)?;
    let last = assignment.nodes.last().copied().unwrap_or_default();
    metadata.set_highest_id(last, assignment.role);
    metadata.set_role_count(assignment.role, assignment.nodes.len() as u32);
    if assignment.is_final_assignment() {
        metadata.fully_assigned = true
    }
    ROLES_METADATA.save(storage, inactive, &metadata)?;

    // set the actual roles
    Ok(ROLES.save(storage, (inactive, assignment.role), &assignment.nodes)?)
}

pub(crate) fn read_rewarded_set_metadata(
    storage: &dyn Storage,
) -> Result<RewardedSetMetadata, MixnetContractError> {
    let active_bucket = ACTIVE_ROLES_BUCKET.load(storage)?;
    Ok(ROLES_METADATA.load(storage, active_bucket as u8)?)
}

pub(crate) fn read_assigned_roles(
    storage: &dyn Storage,
    role: Role,
) -> Result<Vec<NodeId>, MixnetContractError> {
    let active_bucket = ACTIVE_ROLES_BUCKET.load(storage)?;
    // we're always reading from the ACTIVE bucket
    Ok(ROLES.load(storage, (active_bucket as u8, role))?)
}

pub(crate) fn swap_active_role_bucket(
    storage: &mut dyn Storage,
) -> Result<(), MixnetContractError> {
    let active_bucket = ACTIVE_ROLES_BUCKET.load(storage)?;
    Ok(ACTIVE_ROLES_BUCKET.save(storage, &active_bucket.swap())?)
}

pub(crate) fn set_unbonding(
    storage: &mut dyn Storage,
    bond: &NymNodeBond,
) -> Result<(), MixnetContractError> {
    let mut updated_bond = bond.clone();
    updated_bond.is_unbonding = true;
    nym_nodes().replace(storage, bond.node_id, Some(&updated_bond), Some(bond))?;
    Ok(())
}

pub(crate) fn next_nymnode_id_counter(store: &mut dyn Storage) -> StdResult<NodeId> {
    let id: NodeId = NYMNODE_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NYMNODE_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) fn initialise_storage(storage: &mut dyn Storage) -> Result<(), MixnetContractError> {
    ACTIVE_ROLES_BUCKET.save(storage, &RoleStorageBucket::default())?;
    let roles = vec![
        Role::Layer1,
        Role::Layer2,
        Role::Layer3,
        Role::EntryGateway,
        Role::ExitGateway,
        Role::Standby,
    ];
    for role in roles {
        ROLES.save(storage, (RoleStorageBucket::default() as u8, role), &vec![])?;
        ROLES.save(
            storage,
            (RoleStorageBucket::default().other() as u8, role),
            &vec![],
        )?
    }

    ROLES_METADATA.save(
        storage,
        RoleStorageBucket::default() as u8,
        &Default::default(),
    )?;
    ROLES_METADATA.save(
        storage,
        RoleStorageBucket::default().other() as u8,
        &Default::default(),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;

    #[test]
    fn next_id() {
        let mut deps = test_helpers::init_contract();

        for i in 1u32..1000 {
            assert_eq!(i, next_nymnode_id_counter(deps.as_mut().storage).unwrap());
        }
    }
}
