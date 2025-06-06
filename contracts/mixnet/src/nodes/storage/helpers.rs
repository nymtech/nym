// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nodes::storage::rewarded_set::{ACTIVE_ROLES_BUCKET, ROLES, ROLES_METADATA};
use crate::nodes::storage::{nym_nodes, KEY_ROTATION_STATE, NYMNODE_ID_COUNTER};
use cosmwasm_std::{StdResult, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::{RewardedSetMetadata, Role};
use mixnet_contract_common::{EpochId, KeyRotationState, NodeId, NymNodeBond, RoleAssignment};
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
    let highest_id = assignment.nodes.iter().max().copied().unwrap_or_default();
    metadata.set_highest_id(highest_id, assignment.role);
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

pub(crate) fn initialise_storage(
    storage: &mut dyn Storage,
    key_rotation_validity: u32,
) -> Result<(), MixnetContractError> {
    let active_bucket = RoleStorageBucket::default();
    let inactive_bucket = active_bucket.other();

    ACTIVE_ROLES_BUCKET.save(storage, &active_bucket)?;
    let roles = vec![
        Role::Layer1,
        Role::Layer2,
        Role::Layer3,
        Role::EntryGateway,
        Role::ExitGateway,
        Role::Standby,
    ];
    for role in roles {
        ROLES.save(storage, (active_bucket as u8, role), &vec![])?;
        ROLES.save(storage, (inactive_bucket as u8, role), &vec![])?
    }

    ROLES_METADATA.save(storage, active_bucket as u8, &Default::default())?;
    ROLES_METADATA.save(storage, inactive_bucket as u8, &Default::default())?;

    // since we're initialising fresh storage, the current epoch_id is 0
    KEY_ROTATION_STATE.save(
        storage,
        &KeyRotationState {
            validity_epochs: key_rotation_validity,
            initial_epoch_id: 0,
        },
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::TestSetup;

    #[test]
    fn next_id() {
        let mut deps = test_helpers::init_contract();

        for i in 1u32..1000 {
            assert_eq!(i, next_nymnode_id_counter(deps.as_mut().storage).unwrap());
        }
    }

    #[test]
    fn assigning_role_uses_highest_id_even_if_not_sorted() {
        let mut test = TestSetup::new();
        let deps = test.deps_mut();

        let sorted = RoleAssignment {
            role: Role::EntryGateway,
            nodes: vec![1, 2, 3],
        };

        let unsorted = RoleAssignment {
            role: Role::Layer1,
            nodes: vec![8, 5, 4],
        };

        save_assignment(deps.storage, sorted).unwrap();
        save_assignment(deps.storage, unsorted).unwrap();

        let storage = deps.as_ref().storage;

        let active_bucket = ACTIVE_ROLES_BUCKET.load(storage).unwrap();
        let inactive = active_bucket.other() as u8;
        let metadata = ROLES_METADATA.load(storage, inactive).unwrap();

        assert_eq!(metadata.entry_gateway_metadata.highest_id, 3);
        assert_eq!(metadata.layer1_metadata.highest_id, 8);
        assert_eq!(metadata.highest_rewarded_id(), 8)
    }
}
