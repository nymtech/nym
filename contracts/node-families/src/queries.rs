// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{Deps, Env};
use node_families_contract_common::{
    NodeFamiliesContractError, NodeFamilyId, NodeFamilyMembershipResponse, NodeFamilyResponse,
    PendingFamilyInvitationDetails, PendingFamilyInvitationResponse,
};
use nym_mixnet_contract_common::NodeId;

/// Resolve a single family by its id. Returns `family: None` if no family
/// with that id exists.
pub fn query_family_by_id(
    deps: Deps,
    family_id: NodeFamilyId,
) -> Result<NodeFamilyResponse, NodeFamiliesContractError> {
    let family = NodeFamiliesStorage::new()
        .families
        .may_load(deps.storage, family_id)?;
    Ok(NodeFamilyResponse { family_id, family })
}

/// Report which family — if any — a node currently belongs to.
pub fn query_family_membership(
    deps: Deps,
    node_id: NodeId,
) -> Result<NodeFamilyMembershipResponse, NodeFamiliesContractError> {
    let family_id = NodeFamiliesStorage::new()
        .family_members
        .may_load(deps.storage, node_id)?;
    Ok(NodeFamilyMembershipResponse { node_id, family_id })
}

/// Resolve a pending invitation by its composite `(family_id, node_id)` key,
/// stamping it with whether it has already timed out at the current block
/// time so the caller doesn't have to do the comparison itself.
pub fn query_pending_invitation(
    deps: Deps,
    env: Env,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<PendingFamilyInvitationResponse, NodeFamiliesContractError> {
    let now = env.block.time.seconds();
    let invitation = NodeFamiliesStorage::new()
        .pending_family_invitations
        .may_load(deps.storage, (family_id, node_id))?
        .map(|invitation| PendingFamilyInvitationDetails {
            expired: now >= invitation.expires_at,
            invitation,
        });
    Ok(PendingFamilyInvitationResponse {
        family_id,
        node_id,
        invitation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{init_contract_tester, NodeFamiliesContractTesterExt};
    use nym_contracts_common_testing::{ChainOpts, ContractOpts};

    #[cfg(test)]
    mod family_by_id {
        use super::*;

        #[test]
        fn family_by_id_returns_none_when_missing() {
            let tester = init_contract_tester();

            let res = query_family_by_id(tester.deps(), 99).unwrap();
            assert_eq!(res.family_id, 99);
            assert!(res.family.is_none());
        }

        #[test]
        fn family_by_id_returns_persisted_family() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            let res = query_family_by_id(tester.deps(), f.id).unwrap();
            assert_eq!(res.family_id, f.id);
            assert_eq!(res.family, Some(f));
        }
    }

    #[cfg(test)]
    mod family_membership {
        use super::*;

        #[test]
        fn family_membership_returns_none_for_unaffiliated_node() {
            let tester = init_contract_tester();

            let res = query_family_membership(tester.deps(), 999).unwrap();
            assert_eq!(res.node_id, 999);
            assert!(res.family_id.is_none());
        }

        #[test]
        fn family_membership_returns_family_id_for_member() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            tester.add_to_family(f.id, 42);

            let res = query_family_membership(tester.deps(), 42).unwrap();
            assert_eq!(res.node_id, 42);
            assert_eq!(res.family_id, Some(f.id));
        }

        #[test]
        fn family_membership_returns_none_after_node_is_removed() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            tester.add_to_family(f.id, 42);
            tester.remove_from_family(42);

            let res = query_family_membership(tester.deps(), 42).unwrap();
            assert!(res.family_id.is_none());
        }
    }

    #[cfg(test)]
    mod pending_invitation {
        use super::*;

        #[test]
        fn pending_invitation_returns_none_when_missing() {
            let tester = init_contract_tester();
            let env = tester.env();

            let res = query_pending_invitation(tester.deps(), env, 1, 42).unwrap();
            assert_eq!(res.family_id, 1);
            assert_eq!(res.node_id, 42);
            assert!(res.invitation.is_none());
        }

        #[test]
        fn pending_invitation_returns_unexpired_when_in_future() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            let inv = tester.invite_to_family(f.id, 42);
            let env = tester.env();

            let res = query_pending_invitation(tester.deps(), env, f.id, 42).unwrap();
            let details = res.invitation.unwrap();
            assert_eq!(details.invitation, inv);
            assert!(!details.expired);
        }

        #[test]
        fn pending_invitation_flagged_as_expired_after_block_time() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            let expires_at = tester.env().block.time.seconds() + 5;
            tester.invite_to_family_with_expiration(f.id, 42, expires_at);

            // advance block time well past the expiry
            tester.advance_time_by(60);

            let env = tester.env();
            assert!(env.block.time.seconds() >= expires_at);

            let res = query_pending_invitation(tester.deps(), env, f.id, 42).unwrap();
            let details = res.invitation.unwrap();
            assert_eq!(details.invitation.expires_at, expires_at);
            assert!(details.expired);
        }
    }
}
