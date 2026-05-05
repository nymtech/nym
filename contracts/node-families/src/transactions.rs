// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! State-mutating execute handlers. Each entry is currently a stub returning
//! an empty response; concrete implementations will be filled in as the
//! corresponding tickets land.

use crate::helpers::{ensure_address_holds_no_family_membership, normalise_family_name};
use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{DepsMut, Env, Event, MessageInfo, Response};
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
    if let Some((_, existing)) = storage
        .families
        .idx
        .owner
        .item(deps.storage, info.sender.clone())?
    {
        return Err(NodeFamiliesContractError::SenderAlreadyOwnsAFamily {
            address: info.sender,
            family_id: existing.id,
        });
    }

    // explicitly verify duplicate family name for a better error message
    if let Some((_, existing)) = storage
        .families
        .idx
        .name
        .item(deps.storage, normalised.clone())?
    {
        return Err(NodeFamiliesContractError::FamilyNameAlreadyTaken {
            name: normalised,
            family_id: existing.id,
        });
    }

    // check whether this owner has a bonded node which belongs to a family
    ensure_address_holds_no_family_membership(deps.as_ref(), &info.sender)?;

    let family = storage.register_new_family(
        deps.storage,
        &env,
        config.create_family_fee,
        info.sender,
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

pub(crate) fn try_disband_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info);
    Ok(Response::default())
}

pub(crate) fn try_invite_to_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_revoke_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_accept_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, family_id, node_id);
    Ok(Response::default())
}

pub(crate) fn try_reject_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, family_id, node_id);
    Ok(Response::default())
}

pub(crate) fn try_leave_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_kick_from_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
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
        use nym_contracts_common_testing::TEST_DENOM;

        #[test]
        fn happy_path_persists_normalised_family() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let fee = tester.family_fee();
            let alice = tester.make_sender_with_funds("alice", &[fee]);
            let env = tester.env();
            let deps = tester.deps_mut();

            try_create_family(
                deps,
                env,
                alice.clone(),
                "name".to_string(),
                "description".to_string(),
            )?;

            let storage = NodeFamiliesStorage::new();
            let family = storage.families.load(tester.deps().storage, 1)?;
            assert_eq!(family.id, 1);
            assert_eq!(family.name, "name");
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
            let fee = tester.family_fee();
            let alice = tester.make_sender_with_funds("alice", &[fee.clone()]);
            let bob = tester.make_sender_with_funds("bob", &[fee]);
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
        fn rejects_when_owner_owns_node_in_different_family() {
            todo!()
        }
    }
}
