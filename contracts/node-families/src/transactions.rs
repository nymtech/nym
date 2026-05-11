// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! State-mutating execute handlers. Each entry is currently a stub returning
//! an empty response; concrete implementations will be filled in as the
//! corresponding tickets land.

use crate::helpers::{
    ensure_address_holds_no_family_membership, ensure_has_bonded_node, ensure_node_is_bonded,
    ensure_node_not_in_family, normalise_family_name,
};
use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{BankMsg, DepsMut, Env, Event, MessageInfo, Response};
use node_families_contract_common::constants::events;
use node_families_contract_common::{Config, NodeFamiliesContractError, NodeFamilyId};
use nym_mixnet_contract_common::NodeId;

/// Replace the contract's runtime [`Config`]. Restricted to the contract admin.
pub(crate) fn try_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    config: Config,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();
    storage
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;
    storage.config.save(deps.storage, &config)?;
    Ok(Response::default())
}

/// Create a new family owned by `info.sender`.
///
/// Performs the caller-side checks specified on
/// [`NodeFamiliesStorage::register_new_family`]: validates the attached fee
/// matches the configured `create_family_fee`, that name and description
/// are within their configured length limits, that the name normalises to a
/// non-empty string, and that the sender doesn't already own a family or
/// collide with an existing family's normalised name. The unique indexes on
/// `owner` and `name` provide defence-in-depth, but pre-checking yields
/// typed errors with useful context.
pub(crate) fn try_create_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    description: String,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();
    let config = storage.config.load(deps.storage)?;

    // check for the correct number of coins and denom
    let submitted = cw_utils::must_pay(&info, &config.create_family_fee.denom)?;

    // verify the amount
    if submitted != config.create_family_fee.amount {
        return Err(NodeFamiliesContractError::InvalidFamilyCreationFee {
            expected: config.create_family_fee,
            received: info.funds,
        });
    }

    // validate family name
    if name.len() > config.family_name_length_limit {
        return Err(NodeFamiliesContractError::FamilyNameTooLong {
            length: name.len(),
            limit: config.family_name_length_limit,
        });
    }
    let normalised = normalise_family_name(&name);
    if normalised.is_empty() {
        return Err(NodeFamiliesContractError::EmptyFamilyName);
    }

    // validate family description
    if description.len() > config.family_description_length_limit {
        return Err(NodeFamiliesContractError::FamilyDescriptionTooLong {
            length: description.len(),
            limit: config.family_description_length_limit,
        });
    }

    // check if the sender already owns a family
    if let Some(existing) = storage.may_get_owned_family(deps.storage, &info.sender)? {
        return Err(NodeFamiliesContractError::SenderAlreadyOwnsAFamily {
            address: info.sender,
            family_id: existing.id,
        });
    }

    // explicitly verify duplicate family name for a better error message
    if let Some((_, existing)) = storage
        .families
        .idx
        .normalised_name
        .item(deps.storage, normalised.clone())?
    {
        return Err(NodeFamiliesContractError::FamilyNameAlreadyTaken {
            name: normalised,
            family_id: existing.id,
        });
    }

    // check whether this owner has a bonded node which belongs to a family
    ensure_address_holds_no_family_membership(&storage, deps.as_ref(), &info.sender)?;

    let family = storage.register_new_family(
        deps.storage,
        &env,
        config.create_family_fee,
        info.sender,
        name,
        normalised,
        description,
    )?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_CREATION_EVENT_NAME)
            .add_attribute(events::FAMILY_CREATION_EVENT_FAMILY_NAME, family.name)
            .add_attribute(events::FAMILY_CREATION_EVENT_OWNER_ADDRESS, family.owner)
            .add_attribute(
                events::FAMILY_CREATION_EVENT_FAMILY_ID,
                family.id.to_string(),
            ),
    ))
}

/// Disband the family owned by `info.sender` and refund the original
/// creation fee.
///
/// Looks up the sender's family via the `owner` unique index (errors with
/// [`SenderDoesntOwnAFamily`] if none). The storage layer enforces the
/// "family must have zero current members" precondition and sweeps any
/// still-pending invitations as `Revoked`. The originally paid creation fee
/// is returned to the sender via a [`BankMsg::Send`] attached to the
/// response.
///
/// [`SenderDoesntOwnAFamily`]: NodeFamiliesContractError::SenderDoesntOwnAFamily
pub(crate) fn try_disband_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();

    let owned = storage.must_get_owned_family(deps.storage, &info.sender)?;

    if owned.members != 0 {
        return Err(NodeFamiliesContractError::FamilyNotEmpty {
            family_id: owned.id,
            members: owned.members,
        });
    }

    let family = storage.disband_family(deps.storage, &env, owned.id)?;

    let refund = BankMsg::Send {
        to_address: family.owner.to_string(),
        amount: vec![family.paid_fee.clone()],
    };

    Ok(Response::new().add_message(refund).add_event(
        Event::new(events::FAMILY_DISBAND_EVENT_NAME)
            .add_attribute(
                events::FAMILY_DISBAND_EVENT_FAMILY_ID,
                family.id.to_string(),
            )
            .add_attribute(events::FAMILY_DISBAND_EVENT_OWNER_ADDRESS, &family.owner)
            .add_attribute(
                events::FAMILY_DISBAND_EVENT_REFUNDED_FEE,
                family.paid_fee.to_string(),
            ),
    ))
}

/// Issue a pending invitation for `node_id` to join the family owned by
/// `info.sender`.
///
/// `validity_secs` overrides the configured `default_invitation_validity_secs`
/// when supplied; a value of `Some(0)` is rejected with
/// [`ZeroInvitationValidity`] since the invitation would already be expired
/// the moment it landed in storage.
///
/// [`ZeroInvitationValidity`]: NodeFamiliesContractError::ZeroInvitationValidity
pub(crate) fn try_invite_to_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
    validity_secs: Option<u64>,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();
    let config = storage.config.load(deps.storage)?;

    let validity = validity_secs.unwrap_or(config.default_invitation_validity_secs);
    if validity == 0 {
        return Err(NodeFamiliesContractError::ZeroInvitationValidity);
    }

    let owned = storage.must_get_owned_family(deps.storage, &info.sender)?;
    ensure_node_is_bonded(&storage, deps.as_ref(), node_id)?;
    ensure_node_not_in_family(&storage, deps.as_ref(), node_id)?;

    let expires_at = env.block.time.seconds() + validity;
    let invitation = storage.add_pending_invitation(deps.storage, owned.id, node_id, expires_at)?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_INVITATION_EVENT_NAME)
            .add_attribute(
                events::FAMILY_INVITATION_EVENT_FAMILY_ID,
                owned.id.to_string(),
            )
            .add_attribute(events::FAMILY_INVITATION_EVENT_NODE_ID, node_id.to_string())
            .add_attribute(
                events::FAMILY_INVITATION_EVENT_EXPIRES_AT,
                invitation.expires_at.to_string(),
            ),
    ))
}

/// Revoke a still-pending invitation previously issued by the sender's
/// family.
///
/// The sender must currently own a family — the `(family, node)` pair
/// targeted for revocation is derived from that ownership rather than passed
/// explicitly, so a sender cannot revoke another family's invitations.
/// Errors with [`SenderDoesntOwnAFamily`] if the sender owns no family, or
/// [`InvitationNotFound`] if no pending invitation for `node_id` exists in
/// the sender's family. Expired invitations *can* be revoked — this is the
/// only path that cleans them out of the pending map.
///
/// [`SenderDoesntOwnAFamily`]: NodeFamiliesContractError::SenderDoesntOwnAFamily
/// [`InvitationNotFound`]: NodeFamiliesContractError::InvitationNotFound
pub(crate) fn try_revoke_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();
    let owned = storage.must_get_owned_family(deps.storage, &info.sender)?;

    storage.revoke_pending_invitation(deps.storage, &env, owned.id, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_INVITATION_REVOKED_EVENT_NAME)
            .add_attribute(
                events::FAMILY_INVITATION_REVOKED_EVENT_FAMILY_ID,
                owned.id.to_string(),
            )
            .add_attribute(
                events::FAMILY_INVITATION_REVOKED_EVENT_NODE_ID,
                node_id.to_string(),
            ),
    ))
}

/// Accept the pending invitation for `node_id` to join `family_id`.
///
/// `info.sender` must be the bond controller of `node_id` per the mixnet
/// contract; the storage layer's `family_members` write would otherwise
/// silently overwrite the membership of an unrelated node, so the controller
/// check (and the defence-in-depth `ensure_node_not_in_family`) live here
/// rather than down in storage. Errors with [`SenderDoesntControlNode`] if
/// the sender's bonded node id doesn't match (or the node is unbonding),
/// [`NodeAlreadyInFamily`] if the node has somehow joined another family
/// since the invitation was issued, [`InvitationNotFound`] if no pending
/// invitation exists for the pair, and [`InvitationExpired`] if it has.
///
/// [`SenderDoesntControlNode`]: NodeFamiliesContractError::SenderDoesntControlNode
/// [`NodeAlreadyInFamily`]: NodeFamiliesContractError::NodeAlreadyInFamily
/// [`InvitationNotFound`]: NodeFamiliesContractError::InvitationNotFound
/// [`InvitationExpired`]: NodeFamiliesContractError::InvitationExpired
pub(crate) fn try_accept_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();

    ensure_has_bonded_node(&storage, deps.as_ref(), &info.sender, node_id)?;
    ensure_node_not_in_family(&storage, deps.as_ref(), node_id)?;

    storage.accept_invitation(deps.storage, &env, family_id, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_INVITATION_ACCEPTED_EVENT_NAME)
            .add_attribute(
                events::FAMILY_INVITATION_ACCEPTED_EVENT_FAMILY_ID,
                family_id.to_string(),
            )
            .add_attribute(
                events::FAMILY_INVITATION_ACCEPTED_EVENT_NODE_ID,
                node_id.to_string(),
            ),
    ))
}

/// Reject the pending invitation for `node_id` to join `family_id`.
///
/// `info.sender` must be the bond controller of `node_id` per the mixnet
/// contract — rejection is the invitee's choice. Errors with
/// [`SenderDoesntControlNode`] if the sender doesn't control `node_id` (or
/// the node is unbonding) and [`InvitationNotFound`] if no pending invitation
/// exists for the pair. Expired invitations *can* be rejected — symmetric
/// with revocation, this is also a path that cleans them out of the pending
/// map.
///
/// [`SenderDoesntControlNode`]: NodeFamiliesContractError::SenderDoesntControlNode
/// [`InvitationNotFound`]: NodeFamiliesContractError::InvitationNotFound
pub(crate) fn try_reject_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();

    ensure_has_bonded_node(&storage, deps.as_ref(), &info.sender, node_id)?;

    storage.reject_pending_invitation(deps.storage, &env, family_id, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_INVITATION_REJECTED_EVENT_NAME)
            .add_attribute(
                events::FAMILY_INVITATION_REJECTED_EVENT_FAMILY_ID,
                family_id.to_string(),
            )
            .add_attribute(
                events::FAMILY_INVITATION_REJECTED_EVENT_NODE_ID,
                node_id.to_string(),
            ),
    ))
}

/// Remove `node_id` from whichever family it currently belongs to, at the
/// request of the node's controller.
///
/// `info.sender` must be the bond controller of `node_id` per the mixnet
/// contract — a node only leaves of its own accord. Errors with
/// [`SenderDoesntControlNode`] if the sender doesn't control `node_id` (or
/// the node is unbonding) and [`NodeNotInFamily`] if the node isn't currently
/// a member of any family.
///
/// The mixnet-contract unbonding callback drops a node from its family
/// independently (see [`try_handle_node_unbonding`]); this handler is the
/// voluntary-leave path and is not the one fired on unbond.
///
/// [`SenderDoesntControlNode`]: NodeFamiliesContractError::SenderDoesntControlNode
/// [`NodeNotInFamily`]: NodeFamiliesContractError::NodeNotInFamily
pub(crate) fn try_leave_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();

    ensure_has_bonded_node(&storage, deps.as_ref(), &info.sender, node_id)?;

    let family = storage.remove_family_member(deps.storage, &env, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_MEMBER_LEFT_EVENT_NAME)
            .add_attribute(
                events::FAMILY_MEMBER_LEFT_EVENT_FAMILY_ID,
                family.id.to_string(),
            )
            .add_attribute(
                events::FAMILY_MEMBER_LEFT_EVENT_NODE_ID,
                node_id.to_string(),
            ),
    ))
}

/// Kick `node_id` out of the family owned by `info.sender`.
///
/// Owner-gated: the family acted on is derived from `info.sender`'s ownership
/// rather than passed as an argument, so a sender cannot kick from another
/// family. Errors with [`SenderDoesntOwnAFamily`] if the sender owns no
/// family, [`NodeNotInFamily`] if the node has no membership at all, and
/// [`NodeNotMemberOfFamily`] if the node is in a different family — the
/// scope check happens at the tx layer because `family_members` is keyed by
/// node only and `remove_family_member` would otherwise silently strip a
/// node from someone else's family.
///
/// [`SenderDoesntOwnAFamily`]: NodeFamiliesContractError::SenderDoesntOwnAFamily
/// [`NodeNotInFamily`]: NodeFamiliesContractError::NodeNotInFamily
/// [`NodeNotMemberOfFamily`]: NodeFamiliesContractError::NodeNotMemberOfFamily
pub(crate) fn try_kick_from_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();

    let owned = storage.must_get_owned_family(deps.storage, &info.sender)?;

    let membership = storage
        .family_members
        .may_load(deps.storage, node_id)?
        .ok_or(NodeFamiliesContractError::NodeNotInFamily { node_id })?;
    if membership.family_id != owned.id {
        return Err(NodeFamiliesContractError::NodeNotMemberOfFamily {
            node_id,
            family_id: owned.id,
        });
    }

    storage.remove_family_member(deps.storage, &env, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::FAMILY_MEMBER_KICKED_EVENT_NAME)
            .add_attribute(
                events::FAMILY_MEMBER_KICKED_EVENT_FAMILY_ID,
                owned.id.to_string(),
            )
            .add_attribute(
                events::FAMILY_MEMBER_KICKED_EVENT_NODE_ID,
                node_id.to_string(),
            ),
    ))
}

/// Cross-contract callback fired by the mixnet contract the moment `node_id`
/// initiates unbonding. Unbonding is irreversible, so from the families
/// contract's perspective the node is already effectively gone — drop it
/// from any family it currently belongs to and clear every pending
/// invitation issued to it.
///
/// Auth: `info.sender` must equal the configured `mixnet_contract_address`,
/// since the mixnet contract is the only authority that can attest a node
/// has unbonded. Errors with [`UnauthorisedMixnetCallback`] otherwise.
///
/// The membership half is idempotent — a node that initiates unbonding
/// without ever joining a family is the common case and is not an error.
/// Swept invitations are archived as
/// [`FamilyInvitationStatus::Rejected`]: the auto-cleanup shares the
/// `Rejected` terminal state with invitations that would have been
/// explicitly declined by the node controller.
///
/// [`UnauthorisedMixnetCallback`]: NodeFamiliesContractError::UnauthorisedMixnetCallback
pub(crate) fn try_handle_node_unbonding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();

    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    if info.sender != mixnet_contract {
        return Err(NodeFamiliesContractError::UnauthorisedMixnetCallback {
            sender: info.sender,
        });
    }

    storage.handle_node_unbonding(deps.storage, &env, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::NODE_UNBOND_CLEANUP_EVENT_NAME).add_attribute(
            events::NODE_UNBOND_CLEANUP_EVENT_NODE_ID,
            node_id.to_string(),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::init_contract_tester;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::message_info;
    use cw_controllers::AdminError;
    use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};

    fn updated_config() -> Config {
        Config {
            create_family_fee: coin(999, "unym"),
            family_name_length_limit: 1,
            family_description_length_limit: 2,
            default_invitation_validity_secs: 60,
        }
    }

    #[test]
    fn admin_can_replace_the_config() {
        let mut tester = init_contract_tester();
        let admin = tester.admin_msg();
        let new_config = updated_config();
        let env = tester.env();
        let res = try_update_config(tester.deps_mut(), env, admin, new_config.clone());
        assert!(res.is_ok());

        let stored = NodeFamiliesStorage::new()
            .config
            .load(tester.deps().storage)
            .unwrap();
        assert_eq!(stored, new_config);
    }

    #[test]
    fn non_admin_cannot_update_the_config() {
        let mut tester = init_contract_tester();
        let not_admin = tester.generate_account();
        let not_admin = message_info(&not_admin, &[]);

        let original = NodeFamiliesStorage::new()
            .config
            .load(tester.deps().storage)
            .unwrap();

        let env = tester.env();
        let err =
            try_update_config(tester.deps_mut(), env, not_admin, updated_config()).unwrap_err();

        assert_eq!(
            err,
            NodeFamiliesContractError::Admin(AdminError::NotAdmin {})
        );

        // config left untouched
        let stored = NodeFamiliesStorage::new()
            .config
            .load(tester.deps().storage)
            .unwrap();
        assert_eq!(stored, original);
    }

    mod create_family {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use cosmwasm_std::coins;
        use cw_utils::PaymentError;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::TEST_DENOM;

        #[test]
        fn happy_path_persists_family_preserving_submitted_name() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let fee = tester.family_fee();
            let alice = tester.make_sender_with_funds("alice", &[fee]);
            let env = tester.env();
            let deps = tester.deps_mut();

            // user-submitted formatting includes punctuation + casing that
            // the normaliser strips; both forms should end up on the stored
            // record.
            try_create_family(
                deps,
                env,
                alice.clone(),
                "My Family!".to_string(),
                "description".to_string(),
            )?;

            let storage = NodeFamiliesStorage::new();
            let family = storage.families.load(tester.deps().storage, 1)?;
            assert_eq!(family.id, 1);
            assert_eq!(family.name, "My Family!");
            assert_eq!(family.normalised_name, "myfamily");
            assert_eq!(family.owner, alice.sender);
            assert_eq!(family.description, "description");
            assert_eq!(family.paid_fee, alice.funds[0]);
            assert_eq!(family.members, 0);

            Ok(())
        }

        #[test]
        fn rejects_when_no_funds_attached() {
            let mut tester = init_contract_tester();
            let alice = tester.make_sender_with_funds("alice", &[]);
            let env = tester.env();
            let deps = tester.deps_mut();

            let err = try_create_family(
                deps,
                env,
                alice.clone(),
                "name".to_string(),
                "description".to_string(),
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvalidDeposit(PaymentError::NoFunds {})
            );
        }

        #[test]
        fn rejects_when_fee_amount_mismatched() {
            let mut tester = init_contract_tester();
            let fee = tester.family_fee();
            let too_little = coins(fee.amount.u128() - 1, fee.denom.clone());
            let alice = tester.make_sender_with_funds("alice", &too_little);
            let env = tester.env();
            let deps = tester.deps_mut();

            let err = try_create_family(
                deps,
                env,
                alice.clone(),
                "name".to_string(),
                "description".to_string(),
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvalidFamilyCreationFee {
                    expected: fee,
                    received: too_little,
                }
            );
        }

        #[test]
        fn rejects_when_fee_denom_mismatched() {
            let mut tester = init_contract_tester();

            let fee = tester.family_fee();
            let wrong_denom = coins(fee.amount.u128(), "uatom");
            let alice = tester.make_sender_with_funds("alice", &wrong_denom);
            let env = tester.env();
            let deps = tester.deps_mut();

            let err = try_create_family(
                deps,
                env,
                alice.clone(),
                "name".to_string(),
                "description".to_string(),
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvalidDeposit(PaymentError::MissingDenom(
                    TEST_DENOM.to_string()
                ))
            );
        }

        #[test]
        fn rejects_name_exceeding_length_limit() {
            let mut tester = init_contract_tester();

            let fee = tester.family_fee();
            let alice = tester.make_sender_with_funds("alice", &[fee]);

            let limit = NodeFamiliesStorage::new()
                .config
                .load(tester.deps().storage)
                .unwrap()
                .family_name_length_limit;
            let too_long: String = "a".repeat(limit + 1);
            let env = tester.env();
            let deps = tester.deps_mut();

            let err = try_create_family(
                deps,
                env,
                alice,
                too_long.to_string(),
                "description".to_string(),
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::FamilyNameTooLong {
                    length: limit + 1,
                    limit,
                }
            );
        }

        #[test]
        fn rejects_description_exceeding_length_limit() {
            let mut tester = init_contract_tester();

            let fee = tester.family_fee();
            let alice = tester.make_sender_with_funds("alice", &[fee]);

            let limit = NodeFamiliesStorage::new()
                .config
                .load(tester.deps().storage)
                .unwrap()
                .family_description_length_limit;
            let too_long: String = "a".repeat(limit + 1);
            let env = tester.env();
            let deps = tester.deps_mut();

            let err = try_create_family(deps, env, alice, "name".to_string(), too_long.to_string())
                .unwrap_err();

            assert_eq!(
                err,
                NodeFamiliesContractError::FamilyDescriptionTooLong {
                    length: limit + 1,
                    limit,
                }
            );
        }

        #[test]
        fn rejects_name_that_normalises_to_empty() {
            let mut tester = init_contract_tester();
            let env = tester.env();
            let alice = tester.make_sender_with_funds("alice", &[tester.family_fee()]);
            let deps = tester.deps_mut();

            let err = try_create_family(
                deps,
                env,
                alice.clone(),
                "!!! ---".to_string(),
                "".to_string(),
            )
            .unwrap_err();

            assert_eq!(err, NodeFamiliesContractError::EmptyFamilyName);
        }

        #[test]
        fn rejects_when_sender_already_owns_a_family() {
            let mut tester = init_contract_tester();
            let env = tester.env();
            let alice = tester.make_sender_with_funds("alice", &[tester.family_fee()]);

            tester.make_family(&alice.sender);
            let deps = tester.deps_mut();

            let err = try_create_family(
                deps,
                env,
                alice.clone(),
                "name".to_string(),
                "description".to_string(),
            )
            .unwrap_err();

            assert_eq!(
                err,
                NodeFamiliesContractError::SenderAlreadyOwnsAFamily {
                    address: alice.sender,
                    family_id: 1,
                }
            );
        }

        #[test]
        fn rejects_when_normalised_name_is_already_taken() {
            let mut tester = init_contract_tester();
            let fee = vec![tester.family_fee()];
            let alice = tester.make_sender_with_funds("alice", &fee);
            let bob = tester.make_sender_with_funds("bob", &fee);
            let env = tester.env();

            tester.make_named_family(&alice.sender, "MyFamily");
            let deps = tester.deps_mut();

            // different casing / punctuation, same normalised form
            let err = try_create_family(
                deps,
                env,
                bob.clone(),
                "$$myFaMiLy$$".to_string(),
                "description".to_string(),
            )
            .unwrap_err();

            assert_eq!(
                err,
                NodeFamiliesContractError::FamilyNameAlreadyTaken {
                    name: "myfamily".to_string(),
                    family_id: 1,
                }
            );
        }

        #[test]
        fn rejects_when_owner_owns_node_in_different_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let fee = tester.family_fee();
            let alice = tester.generate_account_with_balance();
            let env = tester.env();

            let other_family = tester.make_family(&tester.addr_make("bob"));
            let node_id = tester.bond_dummy_nymnode_for(&alice)?;

            let alice = message_info(&alice, &[fee]);

            // has node which is not in a family - that's still allowed!
            let deps = tester.deps_mut();
            try_create_family(
                deps,
                env.clone(),
                alice.clone(),
                "My Family!".to_string(),
                "description".to_string(),
            )?;
            tester.disband_family(2);

            // after joining family we error out
            tester.add_to_family(other_family.id, node_id);

            let deps = tester.deps_mut();
            let err = try_create_family(
                deps,
                env.clone(),
                alice.clone(),
                "My Family!".to_string(),
                "description".to_string(),
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::AlreadyInFamily {
                    address: alice.sender.clone(),
                    node_id,
                    family_id: other_family.id,
                }
            );

            // after unbonding it is fine again
            tester.unbond_nymnode(node_id)?;
            let deps = tester.deps_mut();
            try_create_family(
                deps,
                env.clone(),
                alice.clone(),
                "My Family!".to_string(),
                "description".to_string(),
            )?;

            Ok(())
        }
    }

    mod disband_family {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use cosmwasm_std::{BankMsg, CosmosMsg, SubMsg};

        #[test]
        fn happy_path_removes_family_and_refunds_fee() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let info = message_info(&alice, &[]);
            let env = tester.env();

            let res = try_disband_family(tester.deps_mut(), env, info)?;

            // family is gone from storage
            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .families
                .may_load(tester.deps().storage, family.id)?
                .is_none());

            // single bank refund attached, going to the original owner with the
            // exact paid fee
            assert_eq!(res.messages.len(), 1);
            assert_eq!(
                res.messages[0],
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: alice.to_string(),
                    amount: vec![family.paid_fee.clone()],
                }))
            );

            Ok(())
        }

        #[test]
        fn rejects_when_sender_owns_no_family() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let info = message_info(&alice, &[]);
            let env = tester.env();

            let err = try_disband_family(tester.deps_mut(), env, info).unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntOwnAFamily { address: alice }
            );
        }

        #[test]
        fn rejects_when_family_still_has_members() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            tester.add_to_family(family.id, 42);

            let info = message_info(&alice, &[]);
            let env = tester.env();
            let err = try_disband_family(tester.deps_mut(), env, info).unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::FamilyNotEmpty {
                    family_id: family.id,
                    members: 1,
                }
            );
        }

        #[test]
        fn after_disband_owner_can_create_a_new_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            tester.make_family(&alice);

            let env = tester.env();
            try_disband_family(tester.deps_mut(), env.clone(), message_info(&alice, &[]))?;

            // owner-index slot freed → a fresh create_family should succeed
            let fee = tester.family_fee();
            let alice_with_fee = message_info(&alice, &[fee]);
            try_create_family(
                tester.deps_mut(),
                env,
                alice_with_fee,
                "second".to_string(),
                "".to_string(),
            )?;
            Ok(())
        }
    }

    mod invite_to_family {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;

        #[test]
        fn happy_path_persists_pending_invitation() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;

            let env = tester.env();
            let info = message_info(&alice, &[]);
            try_invite_to_family(tester.deps_mut(), env.clone(), info, node_id, None)?;

            let storage = NodeFamiliesStorage::new();
            let invitation = storage
                .pending_family_invitations
                .load(tester.deps().storage, (family.id, node_id))?;
            assert_eq!(invitation.family_id, family.id);
            assert_eq!(invitation.node_id, node_id);

            let default_validity = storage
                .config
                .load(tester.deps().storage)?
                .default_invitation_validity_secs;
            assert_eq!(
                invitation.expires_at,
                env.block.time.seconds() + default_validity
            );
            Ok(())
        }

        #[test]
        fn custom_validity_overrides_default() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;

            let env = tester.env();
            let info = message_info(&alice, &[]);
            try_invite_to_family(tester.deps_mut(), env.clone(), info, node_id, Some(5))?;

            let invitation = NodeFamiliesStorage::new()
                .pending_family_invitations
                .load(tester.deps().storage, (family.id, node_id))?;
            assert_eq!(invitation.expires_at, env.block.time.seconds() + 5);
            Ok(())
        }

        #[test]
        fn rejects_zero_validity() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;

            let env = tester.env();
            let info = message_info(&alice, &[]);
            let err =
                try_invite_to_family(tester.deps_mut(), env, info, node_id, Some(0)).unwrap_err();
            assert_eq!(err, NodeFamiliesContractError::ZeroInvitationValidity);
            Ok(())
        }

        #[test]
        fn rejects_when_sender_owns_no_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let node_id = tester.bond_dummy_nymnode()?;

            let env = tester.env();
            let info = message_info(&alice, &[]);
            let err =
                try_invite_to_family(tester.deps_mut(), env, info, node_id, None).unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntOwnAFamily { address: alice }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_node_is_not_bonded() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            tester.make_family(&alice);

            let env = tester.env();
            let info = message_info(&alice, &[]);
            let err = try_invite_to_family(tester.deps_mut(), env, info, 999, None).unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::NodeDoesntExist { node_id: 999 }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_node_is_already_in_a_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let alice_family = tester.make_family(&alice);
            let bob = tester.addr_make("bob");
            let bob_family = tester.make_family(&bob);

            let node_id = tester.bond_dummy_nymnode()?;
            tester.add_to_family(bob_family.id, node_id);

            let env = tester.env();
            let info = message_info(&alice, &[]);
            let err =
                try_invite_to_family(tester.deps_mut(), env, info, node_id, None).unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::NodeAlreadyInFamily {
                    node_id,
                    family_id: bob_family.id,
                }
            );
            // alice's family is unchanged
            assert!(NodeFamiliesStorage::new()
                .pending_family_invitations
                .may_load(tester.deps().storage, (alice_family.id, node_id))?
                .is_none());
            Ok(())
        }

        #[test]
        fn rejects_duplicate_pending_invitation() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;

            let env = tester.env();
            try_invite_to_family(
                tester.deps_mut(),
                env.clone(),
                message_info(&alice, &[]),
                node_id,
                None,
            )?;
            let err = try_invite_to_family(
                tester.deps_mut(),
                env,
                message_info(&alice, &[]),
                node_id,
                None,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::PendingInvitationAlreadyExists {
                    family_id: family.id,
                    node_id,
                }
            );
            Ok(())
        }
    }

    mod revoke_family_invitation {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use node_families_contract_common::FamilyInvitationStatus;

        #[test]
        fn happy_path_removes_pending_and_archives_revoked() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = 7;
            tester.invite_to_family(family.id, node_id);

            let env = tester.env();
            try_revoke_family_invitation(
                tester.deps_mut(),
                env.clone(),
                message_info(&alice, &[]),
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (family.id, node_id))?
                .is_none());

            let archived = storage
                .past_family_invitations
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert!(matches!(
                archived.status,
                FamilyInvitationStatus::Revoked { at } if at == env.block.time.seconds()
            ));
            Ok(())
        }

        #[test]
        fn rejects_when_sender_owns_no_family() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let env = tester.env();

            let err =
                try_revoke_family_invitation(tester.deps_mut(), env, message_info(&alice, &[]), 42)
                    .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntOwnAFamily { address: alice }
            );
        }

        #[test]
        fn rejects_when_no_pending_invitation_for_node() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let env = tester.env();

            let err =
                try_revoke_family_invitation(tester.deps_mut(), env, message_info(&alice, &[]), 42)
                    .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvitationNotFound {
                    family_id: family.id,
                    node_id: 42,
                }
            );
        }

        #[test]
        fn cannot_revoke_another_familys_invitation() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let bob = tester.addr_make("bob");
            tester.make_family(&alice);
            let bob_family = tester.make_family(&bob);
            let node_id = 7;
            tester.invite_to_family(bob_family.id, node_id);

            // alice is targeting node 7 — but there is no pending invitation
            // in *her* family for it, so the lookup misses against alice's id
            let env = tester.env();
            let err = try_revoke_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&alice, &[]),
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvitationNotFound {
                    family_id: 1,
                    node_id,
                }
            );

            // bob's invitation is still pending and untouched
            let still_pending = NodeFamiliesStorage::new()
                .pending_family_invitations
                .may_load(tester.deps().storage, (bob_family.id, node_id))
                .unwrap();
            assert!(still_pending.is_some());
        }

        #[test]
        fn revoking_expired_invitation_is_allowed() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = 7;
            // already-expired (expires_at = 1, well before block.time)
            tester.invite_to_family_with_expiration(family.id, node_id, 1);

            let env = tester.env();
            try_revoke_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&alice, &[]),
                node_id,
            )?;

            // pending entry is gone
            assert!(NodeFamiliesStorage::new()
                .pending_family_invitations
                .may_load(tester.deps().storage, (family.id, node_id))?
                .is_none());
            Ok(())
        }
    }

    mod accept_family_invitation {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use node_families_contract_common::FamilyInvitationStatus;

        #[test]
        fn happy_path_records_membership_and_archives_accepted() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.invite_to_family(family.id, node_id);

            let env = tester.env();
            try_accept_family_invitation(
                tester.deps_mut(),
                env.clone(),
                message_info(&bob, &[]),
                family.id,
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (family.id, node_id))?
                .is_none());

            let membership = storage
                .family_members
                .load(tester.deps().storage, node_id)?;
            assert_eq!(membership.family_id, family.id);
            assert_eq!(membership.joined_at, env.block.time.seconds());

            let updated = storage.families.load(tester.deps().storage, family.id)?;
            assert_eq!(updated.members, 1);

            let archived = storage
                .past_family_invitations
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert!(matches!(
                archived.status,
                FamilyInvitationStatus::Accepted { at } if at == env.block.time.seconds()
            ));
            Ok(())
        }

        #[test]
        fn rejects_when_sender_controls_no_bonded_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;
            tester.invite_to_family(family.id, node_id);

            // mallory doesn't control any bonded node
            let mallory = tester.addr_make("mallory");
            let env = tester.env();
            let err = try_accept_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&mallory, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: mallory,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_sender_controls_a_different_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let bob_node = tester.bond_dummy_nymnode_for(&bob)?;
            // invitation targets a different (also-bonded) node
            let other_node = tester.bond_dummy_nymnode()?;
            tester.invite_to_family(family.id, other_node);

            let env = tester.env();
            let err = try_accept_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                family.id,
                other_node,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: bob,
                    node_id: other_node,
                }
            );
            // sanity: bob really does control bob_node
            assert_ne!(bob_node, other_node);
            Ok(())
        }

        #[test]
        fn rejects_when_sender_node_is_unbonding() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.invite_to_family(family.id, node_id);

            // unbond_nymnode advances the epoch, fully removing the bond from
            // the mixnet store — the ownership query then returns None, which
            // surfaces as the same SenderDoesntControlNode error as
            // "no bonded node at all".
            tester.unbond_nymnode(node_id)?;

            let env = tester.env();
            let err = try_accept_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: bob,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_no_pending_invitation_exists() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;

            let env = tester.env();
            let err = try_accept_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvitationNotFound {
                    family_id: family.id,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_invitation_is_expired() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;

            let env = tester.env();
            // expires_at == now triggers the `now >= expires_at` branch
            tester.invite_to_family_with_expiration(family.id, node_id, env.block.time.seconds());

            let err = try_accept_family_invitation(
                tester.deps_mut(),
                env.clone(),
                message_info(&bob, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvitationExpired {
                    family_id: family.id,
                    node_id,
                    expires_at: env.block.time.seconds(),
                    now: env.block.time.seconds(),
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_node_already_in_another_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let alice_family = tester.make_family(&alice);
            let bob = tester.addr_make("bob");
            let bob_family = tester.make_family(&bob);

            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;

            // bob's node joins bob's family first, then tries to accept
            // alice's still-pending invitation
            tester.invite_to_family(alice_family.id, node_id);
            tester.add_to_family(bob_family.id, node_id);

            let env = tester.env();
            let err = try_accept_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                alice_family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::NodeAlreadyInFamily {
                    node_id,
                    family_id: bob_family.id,
                }
            );

            // membership is unchanged — still in bob's family
            let membership = NodeFamiliesStorage::new()
                .family_members
                .load(tester.deps().storage, node_id)?;
            assert_eq!(membership.family_id, bob_family.id);
            Ok(())
        }
    }

    mod reject_family_invitation {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use node_families_contract_common::FamilyInvitationStatus;

        #[test]
        fn happy_path_removes_pending_and_archives_rejected() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.invite_to_family(family.id, node_id);

            let env = tester.env();
            try_reject_family_invitation(
                tester.deps_mut(),
                env.clone(),
                message_info(&bob, &[]),
                family.id,
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (family.id, node_id))?
                .is_none());

            let archived = storage
                .past_family_invitations
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert!(matches!(
                archived.status,
                FamilyInvitationStatus::Rejected { at } if at == env.block.time.seconds()
            ));

            // membership was never recorded
            assert!(storage
                .family_members
                .may_load(tester.deps().storage, node_id)?
                .is_none());
            Ok(())
        }

        #[test]
        fn rejects_when_sender_controls_no_bonded_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;
            tester.invite_to_family(family.id, node_id);

            let mallory = tester.addr_make("mallory");
            let env = tester.env();
            let err = try_reject_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&mallory, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: mallory,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_sender_controls_a_different_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let bob_node = tester.bond_dummy_nymnode_for(&bob)?;
            let other_node = tester.bond_dummy_nymnode()?;
            tester.invite_to_family(family.id, other_node);

            let env = tester.env();
            let err = try_reject_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                family.id,
                other_node,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: bob,
                    node_id: other_node,
                }
            );
            assert_ne!(bob_node, other_node);
            Ok(())
        }

        #[test]
        fn rejects_when_sender_node_is_unbonding() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.invite_to_family(family.id, node_id);

            tester.unbond_nymnode(node_id)?;

            let env = tester.env();
            let err = try_reject_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: bob,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_no_pending_invitation_exists() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;

            let env = tester.env();
            let err = try_reject_family_invitation(
                tester.deps_mut(),
                env,
                message_info(&bob, &[]),
                family.id,
                node_id,
            )
            .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::InvitationNotFound {
                    family_id: family.id,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejecting_expired_invitation_is_allowed() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;

            let env = tester.env();
            // already-expired (expires_at == now)
            tester.invite_to_family_with_expiration(family.id, node_id, env.block.time.seconds());

            try_reject_family_invitation(
                tester.deps_mut(),
                env.clone(),
                message_info(&bob, &[]),
                family.id,
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (family.id, node_id))?
                .is_none());
            let archived = storage
                .past_family_invitations
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert!(matches!(
                archived.status,
                FamilyInvitationStatus::Rejected { at } if at == env.block.time.seconds()
            ));
            Ok(())
        }
    }

    mod leave_family {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;

        #[test]
        fn happy_path_drops_membership_and_archives_past_member() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.add_to_family(family.id, node_id);

            // sanity: family has the member, count is 1
            let storage = NodeFamiliesStorage::new();
            assert_eq!(
                storage
                    .families
                    .load(tester.deps().storage, family.id)?
                    .members,
                1
            );

            let env = tester.env();
            try_leave_family(
                tester.deps_mut(),
                env.clone(),
                message_info(&bob, &[]),
                node_id,
            )?;

            // membership gone
            assert!(storage
                .family_members
                .may_load(tester.deps().storage, node_id)?
                .is_none());

            // family count decremented
            let updated = storage.families.load(tester.deps().storage, family.id)?;
            assert_eq!(updated.members, 0);

            // archived as past member
            let past = storage
                .past_family_members
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert_eq!(past.family_id, family.id);
            assert_eq!(past.node_id, node_id);
            assert_eq!(past.removed_at, env.block.time.seconds());
            Ok(())
        }

        #[test]
        fn rejects_when_sender_controls_no_bonded_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;
            tester.add_to_family(family.id, node_id);

            let mallory = tester.addr_make("mallory");
            let env = tester.env();
            let err =
                try_leave_family(tester.deps_mut(), env, message_info(&mallory, &[]), node_id)
                    .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: mallory,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_sender_controls_a_different_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let bob_node = tester.bond_dummy_nymnode_for(&bob)?;
            let other_node = tester.bond_dummy_nymnode()?;
            tester.add_to_family(family.id, other_node);

            let env = tester.env();
            let err = try_leave_family(tester.deps_mut(), env, message_info(&bob, &[]), other_node)
                .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: bob,
                    node_id: other_node,
                }
            );
            assert_ne!(bob_node, other_node);
            Ok(())
        }

        #[test]
        fn rejects_when_sender_node_is_unbonding() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.add_to_family(family.id, node_id);

            tester.unbond_nymnode(node_id)?;

            let env = tester.env();
            let err = try_leave_family(tester.deps_mut(), env, message_info(&bob, &[]), node_id)
                .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntControlNode {
                    address: bob,
                    node_id,
                }
            );
            Ok(())
        }

        #[test]
        fn rejects_when_node_is_not_in_any_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;

            let env = tester.env();
            let err = try_leave_family(tester.deps_mut(), env, message_info(&bob, &[]), node_id)
                .unwrap_err();
            assert_eq!(err, NodeFamiliesContractError::NodeNotInFamily { node_id });
            Ok(())
        }
    }

    mod kick_from_family {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;

        #[test]
        fn happy_path_drops_membership_and_archives_past_member() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;
            tester.add_to_family(family.id, node_id);

            let env = tester.env();
            try_kick_from_family(
                tester.deps_mut(),
                env.clone(),
                message_info(&alice, &[]),
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .family_members
                .may_load(tester.deps().storage, node_id)?
                .is_none());

            let updated = storage.families.load(tester.deps().storage, family.id)?;
            assert_eq!(updated.members, 0);

            let past = storage
                .past_family_members
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert_eq!(past.family_id, family.id);
            assert_eq!(past.node_id, node_id);
            assert_eq!(past.removed_at, env.block.time.seconds());
            Ok(())
        }

        #[test]
        fn rejects_when_sender_owns_no_family() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let env = tester.env();

            let err = try_kick_from_family(tester.deps_mut(), env, message_info(&alice, &[]), 42)
                .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::SenderDoesntOwnAFamily { address: alice }
            );
        }

        #[test]
        fn rejects_when_node_is_not_in_any_family() {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            tester.make_family(&alice);
            let env = tester.env();

            let err = try_kick_from_family(tester.deps_mut(), env, message_info(&alice, &[]), 42)
                .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::NodeNotInFamily { node_id: 42 }
            );
        }

        #[test]
        fn cannot_kick_member_of_another_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let bob = tester.addr_make("bob");
            let alice_family = tester.make_family(&alice);
            let bob_family = tester.make_family(&bob);

            let node_id = tester.bond_dummy_nymnode()?;
            tester.add_to_family(bob_family.id, node_id);

            // alice is targeting a node in bob's family — must error rather
            // than silently strip the membership
            let env = tester.env();
            let err =
                try_kick_from_family(tester.deps_mut(), env, message_info(&alice, &[]), node_id)
                    .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::NodeNotMemberOfFamily {
                    node_id,
                    family_id: alice_family.id,
                }
            );

            // bob's membership and family count are untouched
            let storage = NodeFamiliesStorage::new();
            let mem = storage
                .family_members
                .load(tester.deps().storage, node_id)?;
            assert_eq!(mem.family_id, bob_family.id);
            let bob_fam = storage
                .families
                .load(tester.deps().storage, bob_family.id)?;
            assert_eq!(bob_fam.members, 1);
            Ok(())
        }

        #[test]
        fn cannot_kick_member_already_cleared_by_unbond_callback() -> anyhow::Result<()> {
            // The mixnet contract dispatches `OnNymNodeUnbond` to the families
            // contract synchronously when a node initiates unbonding, so by
            // the time `unbond_nymnode` returns the membership is already gone.
            // A subsequent manual kick from the owner has nothing left to act
            // on and surfaces `NodeNotInFamily`.
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);

            let bob = tester.generate_account_with_balance();
            let node_id = tester.bond_dummy_nymnode_for(&bob)?;
            tester.add_to_family(family.id, node_id);
            tester.unbond_nymnode(node_id)?;

            // sanity: the unbond callback already cleaned up the membership
            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .family_members
                .may_load(tester.deps().storage, node_id)?
                .is_none());

            let env = tester.env();
            let err =
                try_kick_from_family(tester.deps_mut(), env, message_info(&alice, &[]), node_id)
                    .unwrap_err();
            assert_eq!(err, NodeFamiliesContractError::NodeNotInFamily { node_id });
            Ok(())
        }
    }

    mod handle_node_unbonding {
        use super::*;
        use crate::testing::NodeFamiliesContractTesterExt;
        use cosmwasm_std::Addr;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use node_families_contract_common::FamilyInvitationStatus;

        fn mixnet_addr(tester: &impl NodeFamiliesContractTesterExt) -> Addr {
            NodeFamiliesStorage::new()
                .mixnet_contract_address
                .load(tester.deps().storage)
                .unwrap()
        }

        #[test]
        fn rejects_when_sender_is_not_the_mixnet_contract() {
            let mut tester = init_contract_tester();
            let mallory = tester.addr_make("mallory");
            let env = tester.env();

            let err =
                try_handle_node_unbonding(tester.deps_mut(), env, message_info(&mallory, &[]), 42)
                    .unwrap_err();
            assert_eq!(
                err,
                NodeFamiliesContractError::UnauthorisedMixnetCallback { sender: mallory }
            );
        }

        #[test]
        fn no_op_when_node_has_no_membership_and_no_invitations() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let env = tester.env();
            let mixnet = mixnet_addr(&tester);

            // node 42 has nothing in the contract — callback succeeds anyway
            try_handle_node_unbonding(tester.deps_mut(), env, message_info(&mixnet, &[]), 42)?;
            Ok(())
        }

        #[test]
        fn drops_membership_when_node_is_a_member() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let family = tester.make_family(&alice);
            let node_id = tester.bond_dummy_nymnode()?;
            tester.add_to_family(family.id, node_id);

            let env = tester.env();
            let mixnet = mixnet_addr(&tester);
            try_handle_node_unbonding(
                tester.deps_mut(),
                env.clone(),
                message_info(&mixnet, &[]),
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            assert!(storage
                .family_members
                .may_load(tester.deps().storage, node_id)?
                .is_none());
            let updated = storage.families.load(tester.deps().storage, family.id)?;
            assert_eq!(updated.members, 0);
            let past = storage
                .past_family_members
                .load(tester.deps().storage, ((family.id, node_id), 0))?;
            assert_eq!(past.removed_at, env.block.time.seconds());
            Ok(())
        }

        #[test]
        fn sweeps_pending_invitations_addressed_to_node() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let bob = tester.addr_make("bob");
            let alice_family = tester.make_family(&alice);
            let bob_family = tester.make_family(&bob);

            let node_id = tester.bond_dummy_nymnode()?;
            tester.invite_to_family(alice_family.id, node_id);
            tester.invite_to_family(bob_family.id, node_id);

            let env = tester.env();
            let mixnet = mixnet_addr(&tester);
            try_handle_node_unbonding(
                tester.deps_mut(),
                env.clone(),
                message_info(&mixnet, &[]),
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            // both pending invitations are gone
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (alice_family.id, node_id))?
                .is_none());
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (bob_family.id, node_id))?
                .is_none());

            // both archived as Rejected
            for fam_id in [alice_family.id, bob_family.id] {
                let past = storage
                    .past_family_invitations
                    .load(tester.deps().storage, ((fam_id, node_id), 0))?;
                assert!(matches!(
                    past.status,
                    FamilyInvitationStatus::Rejected { at } if at == env.block.time.seconds()
                ));
            }
            Ok(())
        }

        #[test]
        fn handles_membership_and_invitation_sweep_together() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let bob = tester.addr_make("bob");
            let alice_family = tester.make_family(&alice);
            let bob_family = tester.make_family(&bob);

            let node_id = tester.bond_dummy_nymnode()?;
            // node is a member of alice's family AND has a pending invite from bob's
            tester.add_to_family(alice_family.id, node_id);
            tester.invite_to_family(bob_family.id, node_id);

            let env = tester.env();
            let mixnet = mixnet_addr(&tester);
            try_handle_node_unbonding(
                tester.deps_mut(),
                env.clone(),
                message_info(&mixnet, &[]),
                node_id,
            )?;

            let storage = NodeFamiliesStorage::new();
            // membership gone
            assert!(storage
                .family_members
                .may_load(tester.deps().storage, node_id)?
                .is_none());
            assert_eq!(
                storage
                    .families
                    .load(tester.deps().storage, alice_family.id)?
                    .members,
                0
            );
            // past member record stamped
            let past_member = storage
                .past_family_members
                .load(tester.deps().storage, ((alice_family.id, node_id), 0))?;
            assert_eq!(past_member.removed_at, env.block.time.seconds());

            // pending invitation from bob's family is swept
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (bob_family.id, node_id))?
                .is_none());
            let archived = storage
                .past_family_invitations
                .load(tester.deps().storage, ((bob_family.id, node_id), 0))?;
            assert!(matches!(
                archived.status,
                FamilyInvitationStatus::Rejected { at } if at == env.block.time.seconds()
            ));
            Ok(())
        }

        #[test]
        fn unrelated_invitations_are_left_untouched() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let alice = tester.addr_make("alice");
            let alice_family = tester.make_family(&alice);

            let unbonding_node = tester.bond_dummy_nymnode()?;
            let other_node = tester.bond_dummy_nymnode()?;
            tester.invite_to_family(alice_family.id, unbonding_node);
            tester.invite_to_family(alice_family.id, other_node);

            let env = tester.env();
            let mixnet = mixnet_addr(&tester);
            try_handle_node_unbonding(
                tester.deps_mut(),
                env,
                message_info(&mixnet, &[]),
                unbonding_node,
            )?;

            let storage = NodeFamiliesStorage::new();
            // unbonding node's invitation is gone
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (alice_family.id, unbonding_node))?
                .is_none());
            // the unrelated invitation is still pending
            assert!(storage
                .pending_family_invitations
                .may_load(tester.deps().storage, (alice_family.id, other_node))?
                .is_some());
            Ok(())
        }
    }
}
