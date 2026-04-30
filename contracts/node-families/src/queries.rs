// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::{retrieval_limits, NodeFamiliesStorage};
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;
use node_families_contract_common::{
    FamiliesPagedResponse, FamilyMemberRecord, FamilyMembersPagedResponse,
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
        .may_load(deps.storage, node_id)?
        .map(|m| m.family_id);
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

/// Page through every node currently in `family_id`, in ascending
/// [`NodeId`] order.
///
/// Backed by the `family` multi-index over [`crate::storage::NodeFamiliesStorage::family_members`],
/// so the cost is O(page size) regardless of how many other families exist.
/// Does not verify that `family_id` refers to an existing family — an
/// unknown id simply yields an empty page.
///
/// `start_after` is exclusive — pass the previous page's `start_next_after`
/// to fetch the next page; pass `None` to start from the lowest-id member.
/// `limit` defaults to [`retrieval_limits::FAMILY_MEMBERS_DEFAULT_LIMIT`]
/// and is clamped to [`retrieval_limits::FAMILY_MEMBERS_MAX_LIMIT`].
pub fn query_family_members_paged(
    deps: Deps,
    family_id: NodeFamilyId,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> Result<FamilyMembersPagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::FAMILY_MEMBERS_DEFAULT_LIMIT)
        .min(retrieval_limits::FAMILY_MEMBERS_MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let storage = NodeFamiliesStorage::new();
    let members = storage
        .family_members
        .idx
        .family
        .prefix(family_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(node_id, membership)| FamilyMemberRecord {
                node_id,
                membership,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = members.last().map(|record| record.node_id);

    Ok(FamilyMembersPagedResponse {
        family_id,
        members,
        start_next_after,
    })
}

/// Page through every existing family in ascending [`NodeFamilyId`] order.
///
/// `start_after` is exclusive — pass the previous page's `start_next_after`
/// to fetch the next page; pass `None` to start from the first family.
/// `limit` defaults to [`retrieval_limits::FAMILIES_DEFAULT_LIMIT`] and is
/// clamped to [`retrieval_limits::FAMILIES_MAX_LIMIT`].
pub fn query_families_paged(
    deps: Deps,
    start_after: Option<NodeFamilyId>,
    limit: Option<u32>,
) -> Result<FamiliesPagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::FAMILIES_DEFAULT_LIMIT)
        .min(retrieval_limits::FAMILIES_MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let storage = NodeFamiliesStorage::new();
    let families = storage
        .families
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = families.last().map(|family| family.id);

    Ok(FamiliesPagedResponse {
        families,
        start_next_after,
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

    #[cfg(test)]
    mod families_paged {
        use super::*;

        #[test]
        fn empty_when_no_families_exist() {
            let tester = init_contract_tester();

            let res = query_families_paged(tester.deps(), None, None).unwrap();
            assert!(res.families.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn returns_all_families_within_default_limit() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();

            let res = query_families_paged(tester.deps(), None, None).unwrap();
            assert_eq!(res.families, vec![f1, f2, f3.clone()]);
            assert_eq!(res.start_next_after, Some(f3.id));
        }

        #[test]
        fn returns_families_in_ascending_id_order() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();

            let res = query_families_paged(tester.deps(), None, None).unwrap();
            let ids: Vec<_> = res.families.iter().map(|f| f.id).collect();
            assert_eq!(ids, vec![f1.id, f2.id, f3.id]);
        }

        #[test]
        fn limit_caps_page_size() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let _f3 = tester.add_dummy_family();

            let res = query_families_paged(tester.deps(), None, Some(2)).unwrap();
            assert_eq!(res.families, vec![f1, f2.clone()]);
            assert_eq!(res.start_next_after, Some(f2.id));
        }

        #[test]
        fn start_after_is_exclusive() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();

            let res = query_families_paged(tester.deps(), Some(f1.id), None).unwrap();
            assert_eq!(res.families, vec![f2, f3.clone()]);
            assert_eq!(res.start_next_after, Some(f3.id));
        }

        #[test]
        fn paginates_through_all_families() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();
            let f4 = tester.add_dummy_family();
            let f5 = tester.add_dummy_family();

            let page1 = query_families_paged(tester.deps(), None, Some(2)).unwrap();
            assert_eq!(page1.families, vec![f1, f2.clone()]);
            assert_eq!(page1.start_next_after, Some(f2.id));

            let page2 =
                query_families_paged(tester.deps(), page1.start_next_after, Some(2)).unwrap();
            assert_eq!(page2.families, vec![f3, f4.clone()]);
            assert_eq!(page2.start_next_after, Some(f4.id));

            let page3 =
                query_families_paged(tester.deps(), page2.start_next_after, Some(2)).unwrap();
            assert_eq!(page3.families, vec![f5.clone()]);
            assert_eq!(page3.start_next_after, Some(f5.id));

            let page4 =
                query_families_paged(tester.deps(), page3.start_next_after, Some(2)).unwrap();
            assert!(page4.families.is_empty());
            assert!(page4.start_next_after.is_none());
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let total = retrieval_limits::FAMILIES_MAX_LIMIT as usize + 5;
            for _ in 0..total {
                tester.add_dummy_family();
            }

            let res = query_families_paged(tester.deps(), None, Some(u32::MAX)).unwrap();
            assert_eq!(
                res.families.len(),
                retrieval_limits::FAMILIES_MAX_LIMIT as usize
            );
        }

        #[test]
        fn default_limit_applied_when_unspecified() {
            let mut tester = init_contract_tester();
            let total = retrieval_limits::FAMILIES_DEFAULT_LIMIT as usize + 5;
            for _ in 0..total {
                tester.add_dummy_family();
            }

            let res = query_families_paged(tester.deps(), None, None).unwrap();
            assert_eq!(
                res.families.len(),
                retrieval_limits::FAMILIES_DEFAULT_LIMIT as usize
            );
        }

        #[test]
        fn start_after_past_end_returns_empty() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            let res = query_families_paged(tester.deps(), Some(f.id), None).unwrap();
            assert!(res.families.is_empty());
            assert!(res.start_next_after.is_none());
        }
    }

    #[cfg(test)]
    mod family_members_paged {
        use super::*;

        #[test]
        fn empty_when_family_has_no_members() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            let res = query_family_members_paged(tester.deps(), f.id, None, None).unwrap();
            assert_eq!(res.family_id, f.id);
            assert!(res.members.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn empty_for_unknown_family_id() {
            let tester = init_contract_tester();

            let res = query_family_members_paged(tester.deps(), 99, None, None).unwrap();
            assert_eq!(res.family_id, 99);
            assert!(res.members.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn returns_only_members_of_requested_family() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();

            tester.add_to_family(f1.id, 10);
            tester.add_to_family(f1.id, 11);
            tester.add_to_family(f2.id, 20);

            let res = query_family_members_paged(tester.deps(), f1.id, None, None).unwrap();
            let ids: Vec<_> = res.members.iter().map(|m| m.node_id).collect();
            assert_eq!(ids, vec![10, 11]);
            for record in &res.members {
                assert_eq!(record.membership.family_id, f1.id);
            }
        }

        #[test]
        fn member_record_carries_joined_at_timestamp() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            tester.add_to_family(f.id, 42);

            let expected = tester.env().block.time.seconds();
            let res = query_family_members_paged(tester.deps(), f.id, None, None).unwrap();
            let record = res.members.into_iter().next().unwrap();
            assert_eq!(record.node_id, 42);
            assert_eq!(record.membership.family_id, f.id);
            assert_eq!(record.membership.joined_at, expected);
        }

        #[test]
        fn members_returned_in_ascending_node_id_order() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            // insert out of order to confirm ordering isn't insertion order
            tester.add_to_family(f.id, 30);
            tester.add_to_family(f.id, 10);
            tester.add_to_family(f.id, 20);

            let res = query_family_members_paged(tester.deps(), f.id, None, None).unwrap();
            let ids: Vec<_> = res.members.iter().map(|m| m.node_id).collect();
            assert_eq!(ids, vec![10, 20, 30]);
        }

        #[test]
        fn limit_caps_page_size() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            for n in [10, 11, 12] {
                tester.add_to_family(f.id, n);
            }

            let res = query_family_members_paged(tester.deps(), f.id, None, Some(2)).unwrap();
            let ids: Vec<_> = res.members.iter().map(|m| m.node_id).collect();
            assert_eq!(ids, vec![10, 11]);
            assert_eq!(res.start_next_after, Some(11));
        }

        #[test]
        fn start_after_is_exclusive() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            for n in [10, 11, 12] {
                tester.add_to_family(f.id, n);
            }

            let res = query_family_members_paged(tester.deps(), f.id, Some(10), None).unwrap();
            let ids: Vec<_> = res.members.iter().map(|m| m.node_id).collect();
            assert_eq!(ids, vec![11, 12]);
            assert_eq!(res.start_next_after, Some(12));
        }

        #[test]
        fn paginates_through_all_members() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            for n in [10, 11, 12, 13, 14] {
                tester.add_to_family(f.id, n);
            }

            let p1 = query_family_members_paged(tester.deps(), f.id, None, Some(2)).unwrap();
            assert_eq!(
                p1.members.iter().map(|m| m.node_id).collect::<Vec<_>>(),
                vec![10, 11]
            );
            assert_eq!(p1.start_next_after, Some(11));

            let p2 =
                query_family_members_paged(tester.deps(), f.id, p1.start_next_after, Some(2))
                    .unwrap();
            assert_eq!(
                p2.members.iter().map(|m| m.node_id).collect::<Vec<_>>(),
                vec![12, 13]
            );
            assert_eq!(p2.start_next_after, Some(13));

            let p3 =
                query_family_members_paged(tester.deps(), f.id, p2.start_next_after, Some(2))
                    .unwrap();
            assert_eq!(
                p3.members.iter().map(|m| m.node_id).collect::<Vec<_>>(),
                vec![14]
            );
            assert_eq!(p3.start_next_after, Some(14));

            let p4 =
                query_family_members_paged(tester.deps(), f.id, p3.start_next_after, Some(2))
                    .unwrap();
            assert!(p4.members.is_empty());
            assert!(p4.start_next_after.is_none());
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let total = retrieval_limits::FAMILY_MEMBERS_MAX_LIMIT as u32 + 5;
            for n in 1..=total {
                tester.add_to_family(f.id, n);
            }

            let res = query_family_members_paged(tester.deps(), f.id, None, Some(u32::MAX))
                .unwrap();
            assert_eq!(
                res.members.len(),
                retrieval_limits::FAMILY_MEMBERS_MAX_LIMIT as usize
            );
        }

        #[test]
        fn excludes_node_after_it_leaves_family() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            tester.add_to_family(f.id, 10);
            tester.add_to_family(f.id, 11);

            tester.remove_from_family(10);

            let res = query_family_members_paged(tester.deps(), f.id, None, None).unwrap();
            let ids: Vec<_> = res.members.iter().map(|m| m.node_id).collect();
            assert_eq!(ids, vec![11]);
        }
    }
}
