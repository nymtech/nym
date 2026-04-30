// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::{retrieval_limits, NodeFamiliesStorage};
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;
use node_families_contract_common::{
    AllPastFamilyInvitationsPagedResponse, FamiliesPagedResponse, FamilyMemberRecord,
    FamilyMembersPagedResponse, GlobalPastFamilyInvitationCursor, NodeFamiliesContractError,
    NodeFamilyId, NodeFamilyMembershipResponse, NodeFamilyResponse, PastFamilyInvitationCursor,
    PastFamilyInvitationForNodeCursor, PastFamilyInvitationsForNodePagedResponse,
    PastFamilyInvitationsPagedResponse,
    PendingFamilyInvitationDetails, PendingFamilyInvitationResponse,
    PendingFamilyInvitationsPagedResponse, PendingInvitationsForNodePagedResponse,
    PendingInvitationsPagedResponse,
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

/// Page through every pending invitation issued by `family_id`, in ascending
/// invitee [`NodeId`] order. Each entry is stamped with `expired` based on
/// the current block time, so callers don't have to compare it themselves.
///
/// Backed by a prefix scan on the composite primary key
/// `(family_id, node_id)` of `pending_family_invitations`, so cost is
/// O(page size). Does not verify that `family_id` refers to an existing
/// family — an unknown id simply yields an empty page.
///
/// `start_after` is exclusive — pass the previous page's `start_next_after`
/// to fetch the next page; pass `None` to start from the lowest-id invitee.
/// `limit` defaults to [`retrieval_limits::PENDING_INVITATIONS_DEFAULT_LIMIT`]
/// and is clamped to [`retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT`].
pub fn query_pending_invitations_for_family_paged(
    deps: Deps,
    env: Env,
    family_id: NodeFamilyId,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> Result<PendingFamilyInvitationsPagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PENDING_INVITATIONS_DEFAULT_LIMIT)
        .min(retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT) as usize;

    let now = env.block.time.seconds();
    let start = start_after.map(Bound::exclusive);

    let storage = NodeFamiliesStorage::new();
    let invitations = storage
        .pending_family_invitations
        .prefix(family_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(_node_id, invitation)| PendingFamilyInvitationDetails {
                expired: now >= invitation.expires_at,
                invitation,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = invitations.last().map(|d| d.invitation.node_id);

    Ok(PendingFamilyInvitationsPagedResponse {
        family_id,
        invitations,
        start_next_after,
    })
}

/// Page through every pending invitation addressed to `node_id`, in ascending
/// issuing [`NodeFamilyId`] order. Each entry is stamped with `expired` based
/// on the current block time.
///
/// Backed by the `node` multi-index over `pending_family_invitations`, so
/// cost is O(page size). `start_after` is exclusive — pass the previous
/// page's `start_next_after` to fetch the next page. `limit` defaults to
/// [`retrieval_limits::PENDING_INVITATIONS_DEFAULT_LIMIT`] and is clamped to
/// [`retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT`].
pub fn query_pending_invitations_for_node_paged(
    deps: Deps,
    env: Env,
    node_id: NodeId,
    start_after: Option<NodeFamilyId>,
    limit: Option<u32>,
) -> Result<PendingInvitationsForNodePagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PENDING_INVITATIONS_DEFAULT_LIMIT)
        .min(retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT) as usize;

    let now = env.block.time.seconds();
    let start = start_after.map(|family_id| Bound::exclusive((family_id, node_id)));

    let storage = NodeFamiliesStorage::new();
    let invitations = storage
        .pending_family_invitations
        .idx
        .node
        .prefix(node_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(_pk, invitation)| PendingFamilyInvitationDetails {
                expired: now >= invitation.expires_at,
                invitation,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = invitations.last().map(|d| d.invitation.family_id);

    Ok(PendingInvitationsForNodePagedResponse {
        node_id,
        invitations,
        start_next_after,
    })
}

/// Page through every pending invitation across all families, in ascending
/// `(family_id, node_id)` order. Each entry is stamped with `expired` based
/// on the current block time.
///
/// Cost is O(page size) — full range scan over the
/// `pending_family_invitations` map without any prefix filter.
///
/// `start_after` is exclusive — pass the previous page's `start_next_after`
/// to fetch the next page; pass `None` to start from the first entry.
/// `limit` defaults to [`retrieval_limits::PENDING_INVITATIONS_DEFAULT_LIMIT`]
/// and is clamped to [`retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT`].
pub fn query_all_pending_invitations_paged(
    deps: Deps,
    env: Env,
    start_after: Option<(NodeFamilyId, NodeId)>,
    limit: Option<u32>,
) -> Result<PendingInvitationsPagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PENDING_INVITATIONS_DEFAULT_LIMIT)
        .min(retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT) as usize;

    let now = env.block.time.seconds();
    let start = start_after.map(Bound::exclusive);

    let storage = NodeFamiliesStorage::new();
    let invitations = storage
        .pending_family_invitations
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(_key, invitation)| PendingFamilyInvitationDetails {
                expired: now >= invitation.expires_at,
                invitation,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = invitations
        .last()
        .map(|d| (d.invitation.family_id, d.invitation.node_id));

    Ok(PendingInvitationsPagedResponse {
        invitations,
        start_next_after,
    })
}

/// Page through every archived (terminal-state) invitation issued by
/// `family_id`, in ascending `(node_id, counter)` order across all
/// `Accepted` / `Rejected` / `Revoked` statuses.
///
/// Uses a direct bounds-based range scan on the primary map keyed by
/// `((family_id, node_id), counter)` — `family_id` is already the leftmost
/// key component, so this avoids the extra storage read per entry that
/// going through the `family` multi-index would incur. Cost is O(page
/// size). Does not verify that `family_id` refers to an existing
/// family — an unknown id simply yields an empty page.
///
/// `start_after` is exclusive — pass the previous page's `start_next_after`
/// to fetch the next page; pass `None` to start from the first archived
/// entry. `limit` defaults to [`retrieval_limits::PAST_INVITATIONS_DEFAULT_LIMIT`]
/// and is clamped to [`retrieval_limits::PAST_INVITATIONS_MAX_LIMIT`].
pub fn query_past_invitations_for_family_paged(
    deps: Deps,
    family_id: NodeFamilyId,
    start_after: Option<PastFamilyInvitationCursor>,
    limit: Option<u32>,
) -> Result<PastFamilyInvitationsPagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PAST_INVITATIONS_DEFAULT_LIMIT)
        .min(retrieval_limits::PAST_INVITATIONS_MAX_LIMIT) as usize;

    let lower =
        start_after.map(|(node_id, counter)| Bound::exclusive(((family_id, node_id), counter)));

    // upper bound = first key of next family;
    let upper = Some(Bound::exclusive(((family_id + 1, 0), 0)));

    let storage = NodeFamiliesStorage::new();
    let entries = storage
        .past_family_invitations
        .range(deps.storage, lower, upper, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = entries
        .last()
        .map(|(((_, node_id), counter), _)| (*node_id, *counter));

    let invitations = entries.into_iter().map(|(_, v)| v).collect();

    Ok(PastFamilyInvitationsPagedResponse {
        family_id,
        invitations,
        start_next_after,
    })
}

/// Page through every archived (terminal-state) invitation addressed to
/// `node_id`, in ascending `(family_id, counter)` order across all
/// `Accepted` / `Rejected` / `Revoked` statuses.
///
/// Backed by the `node` multi-index over `past_family_invitations`, so
/// cost is O(page size). `start_after` is exclusive — pass the previous
/// page's `start_next_after` to fetch the next page. `limit` defaults to
/// [`retrieval_limits::PAST_INVITATIONS_DEFAULT_LIMIT`] and is clamped to
/// [`retrieval_limits::PAST_INVITATIONS_MAX_LIMIT`].
pub fn query_past_invitations_for_node_paged(
    deps: Deps,
    node_id: NodeId,
    start_after: Option<PastFamilyInvitationForNodeCursor>,
    limit: Option<u32>,
) -> Result<PastFamilyInvitationsForNodePagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PAST_INVITATIONS_DEFAULT_LIMIT)
        .min(retrieval_limits::PAST_INVITATIONS_MAX_LIMIT) as usize;

    let start = start_after
        .map(|(family_id, counter)| Bound::exclusive(((family_id, node_id), counter)));

    let storage = NodeFamiliesStorage::new();
    let entries = storage
        .past_family_invitations
        .idx
        .node
        .prefix(node_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = entries
        .last()
        .map(|(((family_id, _), counter), _)| (*family_id, *counter));

    let invitations = entries.into_iter().map(|(_, v)| v).collect();

    Ok(PastFamilyInvitationsForNodePagedResponse {
        node_id,
        invitations,
        start_next_after,
    })
}

/// Page through every archived (terminal-state) invitation across all
/// families, in ascending `((family_id, node_id), counter)` order.
///
/// Cost is O(page size) — full range scan over the
/// `past_family_invitations` map without any prefix filter.
///
/// `start_after` is exclusive — pass the previous page's `start_next_after`
/// to fetch the next page; pass `None` to start from the first entry.
/// `limit` defaults to [`retrieval_limits::PAST_INVITATIONS_DEFAULT_LIMIT`]
/// and is clamped to [`retrieval_limits::PAST_INVITATIONS_MAX_LIMIT`].
pub fn query_all_past_invitations_paged(
    deps: Deps,
    start_after: Option<GlobalPastFamilyInvitationCursor>,
    limit: Option<u32>,
) -> Result<AllPastFamilyInvitationsPagedResponse, NodeFamiliesContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PAST_INVITATIONS_DEFAULT_LIMIT)
        .min(retrieval_limits::PAST_INVITATIONS_MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let storage = NodeFamiliesStorage::new();
    let entries = storage
        .past_family_invitations
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = entries.last().map(|(key, _)| *key);

    let invitations = entries.into_iter().map(|(_, v)| v).collect();

    Ok(AllPastFamilyInvitationsPagedResponse {
        invitations,
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

            let p2 = query_family_members_paged(tester.deps(), f.id, p1.start_next_after, Some(2))
                .unwrap();
            assert_eq!(
                p2.members.iter().map(|m| m.node_id).collect::<Vec<_>>(),
                vec![12, 13]
            );
            assert_eq!(p2.start_next_after, Some(13));

            let p3 = query_family_members_paged(tester.deps(), f.id, p2.start_next_after, Some(2))
                .unwrap();
            assert_eq!(
                p3.members.iter().map(|m| m.node_id).collect::<Vec<_>>(),
                vec![14]
            );
            assert_eq!(p3.start_next_after, Some(14));

            let p4 = query_family_members_paged(tester.deps(), f.id, p3.start_next_after, Some(2))
                .unwrap();
            assert!(p4.members.is_empty());
            assert!(p4.start_next_after.is_none());
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let total = retrieval_limits::FAMILY_MEMBERS_MAX_LIMIT + 5;
            tester.add_n_family_members(f.id, total);

            let res =
                query_family_members_paged(tester.deps(), f.id, None, Some(u32::MAX)).unwrap();
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

    #[cfg(test)]
    mod pending_invitations_for_family_paged {
        use super::*;

        #[test]
        fn empty_when_family_has_no_pending_invitations() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let env = tester.env();

            let res =
                query_pending_invitations_for_family_paged(tester.deps(), env, f.id, None, None)
                    .unwrap();
            assert_eq!(res.family_id, f.id);
            assert!(res.invitations.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn empty_for_unknown_family_id() {
            let tester = init_contract_tester();
            let env = tester.env();

            let res =
                query_pending_invitations_for_family_paged(tester.deps(), env, 99, None, None)
                    .unwrap();
            assert_eq!(res.family_id, 99);
            assert!(res.invitations.is_empty());
        }

        #[test]
        fn returns_only_invitations_from_requested_family() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.invite_to_family(f1.id, 10);
            tester.invite_to_family(f1.id, 11);
            tester.invite_to_family(f2.id, 20);

            let env = tester.env();
            let res =
                query_pending_invitations_for_family_paged(tester.deps(), env, f1.id, None, None)
                    .unwrap();
            let ids: Vec<_> = res
                .invitations
                .iter()
                .map(|d| d.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![10, 11]);
            for d in &res.invitations {
                assert_eq!(d.invitation.family_id, f1.id);
            }
        }

        #[test]
        fn returned_in_ascending_node_id_order() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            // out-of-order inserts
            tester.invite_to_family(f.id, 30);
            tester.invite_to_family(f.id, 10);
            tester.invite_to_family(f.id, 20);

            let env = tester.env();
            let res =
                query_pending_invitations_for_family_paged(tester.deps(), env, f.id, None, None)
                    .unwrap();
            let ids: Vec<_> = res
                .invitations
                .iter()
                .map(|d| d.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![10, 20, 30]);
        }

        #[test]
        fn flags_expired_against_current_block_time() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let now = tester.env().block.time.seconds();

            tester.invite_to_family_with_expiration(f.id, 10, now + 5);
            tester.invite_to_family_with_expiration(f.id, 11, now + 1000);

            tester.advance_time_by(60);
            let env = tester.env();
            let res =
                query_pending_invitations_for_family_paged(tester.deps(), env, f.id, None, None)
                    .unwrap();

            assert_eq!(res.invitations[0].invitation.node_id, 10);
            assert!(res.invitations[0].expired);
            assert_eq!(res.invitations[1].invitation.node_id, 11);
            assert!(!res.invitations[1].expired);
        }

        #[test]
        fn limit_caps_page_size_and_start_after_is_exclusive() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            for n in [10, 11, 12] {
                tester.invite_to_family(f.id, n);
            }

            let env = tester.env();
            let p1 = query_pending_invitations_for_family_paged(
                tester.deps(),
                env.clone(),
                f.id,
                None,
                Some(2),
            )
            .unwrap();
            let ids: Vec<_> = p1
                .invitations
                .iter()
                .map(|d| d.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![10, 11]);
            assert_eq!(p1.start_next_after, Some(11));

            let p2 = query_pending_invitations_for_family_paged(
                tester.deps(),
                env,
                f.id,
                p1.start_next_after,
                Some(2),
            )
            .unwrap();
            let ids: Vec<_> = p2
                .invitations
                .iter()
                .map(|d| d.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![12]);
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let total = retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT + 5;
            for n in 1..=total {
                tester.invite_to_family(f.id, n);
            }

            let env = tester.env();
            let res = query_pending_invitations_for_family_paged(
                tester.deps(),
                env,
                f.id,
                None,
                Some(u32::MAX),
            )
            .unwrap();
            assert_eq!(
                res.invitations.len(),
                retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT as usize
            );
        }

        #[test]
        fn excludes_invitation_after_it_is_revoked() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            tester.invite_to_family(f.id, 10);
            tester.invite_to_family(f.id, 11);

            // accepting moves the invitation out of the pending map
            tester.accept_invitation(f.id, 10);

            let env = tester.env();
            let res =
                query_pending_invitations_for_family_paged(tester.deps(), env, f.id, None, None)
                    .unwrap();
            let ids: Vec<_> = res
                .invitations
                .iter()
                .map(|d| d.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![11]);
        }
    }

    #[cfg(test)]
    mod all_pending_invitations_paged {
        use super::*;

        #[test]
        fn empty_when_no_pending_invitations() {
            let tester = init_contract_tester();
            let env = tester.env();

            let res = query_all_pending_invitations_paged(tester.deps(), env, None, None).unwrap();
            assert!(res.invitations.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn returns_invitations_across_all_families() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.invite_to_family(f1.id, 10);
            tester.invite_to_family(f1.id, 20);
            tester.invite_to_family(f2.id, 5);

            let env = tester.env();
            let res = query_all_pending_invitations_paged(tester.deps(), env, None, None).unwrap();

            let pairs: Vec<_> = res
                .invitations
                .iter()
                .map(|d| (d.invitation.family_id, d.invitation.node_id))
                .collect();
            // ordered by (family_id asc, node_id asc)
            assert_eq!(pairs, vec![(f1.id, 10), (f1.id, 20), (f2.id, 5)]);
            assert_eq!(res.start_next_after, Some((f2.id, 5)));
        }

        #[test]
        fn flags_expired_against_current_block_time() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let now = tester.env().block.time.seconds();
            tester.invite_to_family_with_expiration(f.id, 10, now + 5);
            tester.invite_to_family_with_expiration(f.id, 11, now + 1000);

            tester.advance_time_by(60);
            let env = tester.env();
            let res = query_all_pending_invitations_paged(tester.deps(), env, None, None).unwrap();

            let by_node: std::collections::HashMap<_, _> = res
                .invitations
                .iter()
                .map(|d| (d.invitation.node_id, d.expired))
                .collect();
            assert!(by_node[&10]);
            assert!(!by_node[&11]);
        }

        #[test]
        fn paginates_with_composite_cursor() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.invite_to_family(f1.id, 10);
            tester.invite_to_family(f1.id, 20);
            tester.invite_to_family(f2.id, 5);
            tester.invite_to_family(f2.id, 15);

            let env = tester.env();
            let p1 = query_all_pending_invitations_paged(tester.deps(), env.clone(), None, Some(2))
                .unwrap();
            let pairs: Vec<_> = p1
                .invitations
                .iter()
                .map(|d| (d.invitation.family_id, d.invitation.node_id))
                .collect();
            assert_eq!(pairs, vec![(f1.id, 10), (f1.id, 20)]);
            assert_eq!(p1.start_next_after, Some((f1.id, 20)));

            let p2 = query_all_pending_invitations_paged(
                tester.deps(),
                env.clone(),
                p1.start_next_after,
                Some(2),
            )
            .unwrap();
            let pairs: Vec<_> = p2
                .invitations
                .iter()
                .map(|d| (d.invitation.family_id, d.invitation.node_id))
                .collect();
            assert_eq!(pairs, vec![(f2.id, 5), (f2.id, 15)]);

            let p3 = query_all_pending_invitations_paged(
                tester.deps(),
                env,
                p2.start_next_after,
                Some(2),
            )
            .unwrap();
            assert!(p3.invitations.is_empty());
            assert!(p3.start_next_after.is_none());
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let total = retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT + 5;
            for n in 1..=total {
                tester.invite_to_family(f.id, n);
            }

            let env = tester.env();
            let res = query_all_pending_invitations_paged(tester.deps(), env, None, Some(u32::MAX))
                .unwrap();
            assert_eq!(
                res.invitations.len(),
                retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT as usize
            );
        }
    }

    #[cfg(test)]
    mod past_invitations_for_family_paged {
        use super::*;
        use node_families_contract_common::FamilyInvitationStatus;

        #[test]
        fn empty_when_family_has_no_archive_entries() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            let res =
                query_past_invitations_for_family_paged(tester.deps(), f.id, None, None).unwrap();
            assert_eq!(res.family_id, f.id);
            assert!(res.invitations.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn empty_for_unknown_family_id() {
            let tester = init_contract_tester();

            let res =
                query_past_invitations_for_family_paged(tester.deps(), 99, None, None).unwrap();
            assert_eq!(res.family_id, 99);
            assert!(res.invitations.is_empty());
        }

        #[test]
        fn returns_only_archived_invitations_from_requested_family() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            // produce one Accepted in each family, plus one Rejected in f1
            tester.add_to_family(f1.id, 10);
            tester.invite_to_family(f1.id, 11);
            tester.reject_invitation(f1.id, 11);
            tester.add_to_family(f2.id, 20);

            let res =
                query_past_invitations_for_family_paged(tester.deps(), f1.id, None, None).unwrap();
            assert_eq!(res.invitations.len(), 2);
            for entry in &res.invitations {
                assert_eq!(entry.invitation.family_id, f1.id);
            }
        }

        #[test]
        fn covers_all_terminal_statuses() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            // Accepted
            tester.add_to_family(f.id, 10);
            // Rejected
            tester.invite_to_family(f.id, 11);
            tester.reject_invitation(f.id, 11);
            // Revoked
            tester.invite_to_family(f.id, 12);
            tester.revoke_invitation(f.id, 12);

            let res =
                query_past_invitations_for_family_paged(tester.deps(), f.id, None, None).unwrap();
            let by_node: std::collections::HashMap<_, _> = res
                .invitations
                .iter()
                .map(|p| (p.invitation.node_id, p.status.clone()))
                .collect();
            assert!(matches!(
                by_node[&10],
                FamilyInvitationStatus::Accepted { .. }
            ));
            assert!(matches!(
                by_node[&11],
                FamilyInvitationStatus::Rejected { .. }
            ));
            assert!(matches!(
                by_node[&12],
                FamilyInvitationStatus::Revoked { .. }
            ));
        }

        #[test]
        fn ordered_by_node_id_then_counter() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();

            // node 42 joins and leaves twice — produces two Accepted entries with counters 0 and 1
            for _ in 0..2 {
                tester.add_to_family(f.id, 42);
                tester.remove_from_family(42);
            }
            // node 7 has one Accepted entry
            tester.add_to_family(f.id, 7);

            let res =
                query_past_invitations_for_family_paged(tester.deps(), f.id, None, None).unwrap();
            let pairs: Vec<_> = res
                .invitations
                .iter()
                .map(|p| p.invitation.node_id)
                .collect();
            // 7 comes before 42; 42's two entries come together (both with counter 0 and 1)
            assert_eq!(pairs, vec![7, 42, 42]);
        }

        #[test]
        fn paginates_with_node_counter_cursor() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            for n in [10, 11, 12] {
                tester.add_to_family(f.id, n);
            }

            let p1 = query_past_invitations_for_family_paged(tester.deps(), f.id, None, Some(2))
                .unwrap();
            let ids: Vec<_> = p1
                .invitations
                .iter()
                .map(|p| p.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![10, 11]);
            assert_eq!(p1.start_next_after, Some((11, 0)));

            let p2 = query_past_invitations_for_family_paged(
                tester.deps(),
                f.id,
                p1.start_next_after,
                Some(2),
            )
            .unwrap();
            let ids: Vec<_> = p2
                .invitations
                .iter()
                .map(|p| p.invitation.node_id)
                .collect();
            assert_eq!(ids, vec![12]);
            assert_eq!(p2.start_next_after, Some((12, 0)));

            let p3 = query_past_invitations_for_family_paged(
                tester.deps(),
                f.id,
                p2.start_next_after,
                Some(2),
            )
            .unwrap();
            assert!(p3.invitations.is_empty());
            assert!(p3.start_next_after.is_none());
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let total = retrieval_limits::PAST_INVITATIONS_MAX_LIMIT + 5;
            tester.add_n_family_members(f.id, total);

            let res =
                query_past_invitations_for_family_paged(tester.deps(), f.id, None, Some(u32::MAX))
                    .unwrap();
            assert_eq!(
                res.invitations.len(),
                retrieval_limits::PAST_INVITATIONS_MAX_LIMIT as usize
            );
        }
    }

    #[cfg(test)]
    mod all_past_invitations_paged {
        use super::*;

        #[test]
        fn empty_when_no_archive_entries() {
            let tester = init_contract_tester();

            let res = query_all_past_invitations_paged(tester.deps(), None, None).unwrap();
            assert!(res.invitations.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn returns_archives_across_all_families() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.add_to_family(f1.id, 10);
            tester.add_to_family(f1.id, 20);
            tester.add_to_family(f2.id, 5);

            let res = query_all_past_invitations_paged(tester.deps(), None, None).unwrap();
            let pairs: Vec<_> = res
                .invitations
                .iter()
                .map(|p| (p.invitation.family_id, p.invitation.node_id))
                .collect();
            assert_eq!(pairs, vec![(f1.id, 10), (f1.id, 20), (f2.id, 5)]);
            assert_eq!(res.start_next_after, Some(((f2.id, 5), 0)));
        }

        #[test]
        fn paginates_with_composite_cursor() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.add_to_family(f1.id, 10);
            tester.add_to_family(f1.id, 20);
            tester.add_to_family(f2.id, 5);
            tester.add_to_family(f2.id, 15);

            let p1 = query_all_past_invitations_paged(tester.deps(), None, Some(2)).unwrap();
            let pairs: Vec<_> = p1
                .invitations
                .iter()
                .map(|p| (p.invitation.family_id, p.invitation.node_id))
                .collect();
            assert_eq!(pairs, vec![(f1.id, 10), (f1.id, 20)]);
            assert_eq!(p1.start_next_after, Some(((f1.id, 20), 0)));

            let p2 = query_all_past_invitations_paged(tester.deps(), p1.start_next_after, Some(2))
                .unwrap();
            let pairs: Vec<_> = p2
                .invitations
                .iter()
                .map(|p| (p.invitation.family_id, p.invitation.node_id))
                .collect();
            assert_eq!(pairs, vec![(f2.id, 5), (f2.id, 15)]);

            let p3 = query_all_past_invitations_paged(tester.deps(), p2.start_next_after, Some(2))
                .unwrap();
            assert!(p3.invitations.is_empty());
            assert!(p3.start_next_after.is_none());
        }

        #[test]
        fn per_pair_counter_disambiguates_repeat_archive_entries() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            // node 42 joins and leaves twice — two Accepted entries for (f, 42) with counters 0 and 1
            for _ in 0..2 {
                tester.add_to_family(f.id, 42);
                tester.remove_from_family(42);
            }

            let res = query_all_past_invitations_paged(tester.deps(), None, None).unwrap();
            assert_eq!(res.invitations.len(), 2);
            assert_eq!(res.start_next_after, Some(((f.id, 42), 1)));
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let f = tester.add_dummy_family();
            let total = retrieval_limits::PAST_INVITATIONS_MAX_LIMIT + 5;
            tester.add_n_family_members(f.id, total);

            let res =
                query_all_past_invitations_paged(tester.deps(), None, Some(u32::MAX)).unwrap();
            assert_eq!(
                res.invitations.len(),
                retrieval_limits::PAST_INVITATIONS_MAX_LIMIT as usize
            );
        }
    }

    #[cfg(test)]
    mod pending_invitations_for_node_paged {
        use super::*;

        #[test]
        fn empty_when_node_has_no_pending_invitations() {
            let tester = init_contract_tester();
            let env = tester.env();

            let res =
                query_pending_invitations_for_node_paged(tester.deps(), env, 42, None, None)
                    .unwrap();
            assert_eq!(res.node_id, 42);
            assert!(res.invitations.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn returns_only_invitations_for_requested_node() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.invite_to_family(f1.id, 10);
            tester.invite_to_family(f1.id, 11);
            tester.invite_to_family(f2.id, 10);

            let env = tester.env();
            let res =
                query_pending_invitations_for_node_paged(tester.deps(), env, 10, None, None)
                    .unwrap();
            let pairs: Vec<_> = res
                .invitations
                .iter()
                .map(|d| (d.invitation.family_id, d.invitation.node_id))
                .collect();
            assert_eq!(pairs, vec![(f1.id, 10), (f2.id, 10)]);
        }

        #[test]
        fn ordered_by_ascending_family_id() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();
            // out-of-order inserts
            tester.invite_to_family(f3.id, 7);
            tester.invite_to_family(f1.id, 7);
            tester.invite_to_family(f2.id, 7);

            let env = tester.env();
            let res =
                query_pending_invitations_for_node_paged(tester.deps(), env, 7, None, None)
                    .unwrap();
            let ids: Vec<_> = res
                .invitations
                .iter()
                .map(|d| d.invitation.family_id)
                .collect();
            assert_eq!(ids, vec![f1.id, f2.id, f3.id]);
        }

        #[test]
        fn paginates_with_family_id_cursor() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();
            tester.invite_to_family(f1.id, 7);
            tester.invite_to_family(f2.id, 7);
            tester.invite_to_family(f3.id, 7);

            let env = tester.env();
            let p1 = query_pending_invitations_for_node_paged(
                tester.deps(),
                env.clone(),
                7,
                None,
                Some(2),
            )
            .unwrap();
            let ids: Vec<_> = p1
                .invitations
                .iter()
                .map(|d| d.invitation.family_id)
                .collect();
            assert_eq!(ids, vec![f1.id, f2.id]);
            assert_eq!(p1.start_next_after, Some(f2.id));

            let p2 = query_pending_invitations_for_node_paged(
                tester.deps(),
                env.clone(),
                7,
                p1.start_next_after,
                Some(2),
            )
            .unwrap();
            let ids: Vec<_> = p2
                .invitations
                .iter()
                .map(|d| d.invitation.family_id)
                .collect();
            assert_eq!(ids, vec![f3.id]);
            assert_eq!(p2.start_next_after, Some(f3.id));

            let p3 = query_pending_invitations_for_node_paged(
                tester.deps(),
                env,
                7,
                p2.start_next_after,
                Some(2),
            )
            .unwrap();
            assert!(p3.invitations.is_empty());
            assert!(p3.start_next_after.is_none());
        }

        #[test]
        fn flags_expired_against_current_block_time() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let now = tester.env().block.time.seconds();
            tester.invite_to_family_with_expiration(f1.id, 7, now + 5);
            tester.invite_to_family_with_expiration(f2.id, 7, now + 1000);

            tester.advance_time_by(60);
            let env = tester.env();
            let res =
                query_pending_invitations_for_node_paged(tester.deps(), env, 7, None, None)
                    .unwrap();
            let by_family: std::collections::HashMap<_, _> = res
                .invitations
                .iter()
                .map(|d| (d.invitation.family_id, d.expired))
                .collect();
            assert!(by_family[&f1.id]);
            assert!(!by_family[&f2.id]);
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let total = retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT + 5;
            for _ in 0..total {
                let f = tester.add_dummy_family();
                tester.invite_to_family(f.id, 7);
            }

            let env = tester.env();
            let res = query_pending_invitations_for_node_paged(
                tester.deps(),
                env,
                7,
                None,
                Some(u32::MAX),
            )
            .unwrap();
            assert_eq!(
                res.invitations.len(),
                retrieval_limits::PENDING_INVITATIONS_MAX_LIMIT as usize
            );
        }
    }

    #[cfg(test)]
    mod past_invitations_for_node_paged {
        use super::*;
        use node_families_contract_common::FamilyInvitationStatus;

        #[test]
        fn empty_when_node_has_no_archive_entries() {
            let tester = init_contract_tester();

            let res =
                query_past_invitations_for_node_paged(tester.deps(), 42, None, None).unwrap();
            assert_eq!(res.node_id, 42);
            assert!(res.invitations.is_empty());
            assert!(res.start_next_after.is_none());
        }

        #[test]
        fn returns_only_archives_for_requested_node() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            tester.add_to_family(f1.id, 7);
            tester.add_to_family(f1.id, 8);
            tester.add_to_family(f2.id, 7);

            let res =
                query_past_invitations_for_node_paged(tester.deps(), 7, None, None).unwrap();
            assert_eq!(res.invitations.len(), 2);
            for entry in &res.invitations {
                assert_eq!(entry.invitation.node_id, 7);
            }
        }

        #[test]
        fn covers_all_terminal_statuses() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();
            // Accepted in f1
            tester.add_to_family(f1.id, 7);
            // Rejected in f2
            tester.invite_to_family(f2.id, 7);
            tester.reject_invitation(f2.id, 7);
            // Revoked in f3
            tester.invite_to_family(f3.id, 7);
            tester.revoke_invitation(f3.id, 7);

            let res =
                query_past_invitations_for_node_paged(tester.deps(), 7, None, None).unwrap();
            let by_family: std::collections::HashMap<_, _> = res
                .invitations
                .iter()
                .map(|p| (p.invitation.family_id, p.status.clone()))
                .collect();
            assert!(matches!(
                by_family[&f1.id],
                FamilyInvitationStatus::Accepted { .. }
            ));
            assert!(matches!(
                by_family[&f2.id],
                FamilyInvitationStatus::Rejected { .. }
            ));
            assert!(matches!(
                by_family[&f3.id],
                FamilyInvitationStatus::Revoked { .. }
            ));
        }

        #[test]
        fn ordered_by_family_id_then_counter() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            // node 7 joins and leaves f1 twice — counters 0 and 1 in f1
            for _ in 0..2 {
                tester.add_to_family(f1.id, 7);
                tester.remove_from_family(7);
            }
            // one Accepted in f2
            tester.add_to_family(f2.id, 7);

            let res =
                query_past_invitations_for_node_paged(tester.deps(), 7, None, None).unwrap();
            let pairs: Vec<_> = res
                .invitations
                .iter()
                .map(|p| p.invitation.family_id)
                .collect();
            assert_eq!(pairs, vec![f1.id, f1.id, f2.id]);
        }

        #[test]
        fn paginates_with_family_counter_cursor() {
            let mut tester = init_contract_tester();
            let f1 = tester.add_dummy_family();
            let f2 = tester.add_dummy_family();
            let f3 = tester.add_dummy_family();
            tester.add_to_family(f1.id, 7);
            tester.add_to_family(f2.id, 7);
            tester.add_to_family(f3.id, 7);

            let p1 =
                query_past_invitations_for_node_paged(tester.deps(), 7, None, Some(2)).unwrap();
            let ids: Vec<_> = p1
                .invitations
                .iter()
                .map(|p| p.invitation.family_id)
                .collect();
            assert_eq!(ids, vec![f1.id, f2.id]);
            assert_eq!(p1.start_next_after, Some((f2.id, 0)));

            let p2 = query_past_invitations_for_node_paged(
                tester.deps(),
                7,
                p1.start_next_after,
                Some(2),
            )
            .unwrap();
            let ids: Vec<_> = p2
                .invitations
                .iter()
                .map(|p| p.invitation.family_id)
                .collect();
            assert_eq!(ids, vec![f3.id]);
            assert_eq!(p2.start_next_after, Some((f3.id, 0)));

            let p3 = query_past_invitations_for_node_paged(
                tester.deps(),
                7,
                p2.start_next_after,
                Some(2),
            )
            .unwrap();
            assert!(p3.invitations.is_empty());
            assert!(p3.start_next_after.is_none());
        }

        #[test]
        fn limit_is_clamped_to_max() {
            let mut tester = init_contract_tester();
            let total = retrieval_limits::PAST_INVITATIONS_MAX_LIMIT + 5;
            for _ in 0..total {
                let f = tester.add_dummy_family();
                tester.add_to_family(f.id, 7);
                tester.remove_from_family(7);
            }

            let res = query_past_invitations_for_node_paged(
                tester.deps(),
                7,
                None,
                Some(u32::MAX),
            )
            .unwrap();
            assert_eq!(
                res.invitations.len(),
                retrieval_limits::PAST_INVITATIONS_MAX_LIMIT as usize
            );
        }
    }
}
