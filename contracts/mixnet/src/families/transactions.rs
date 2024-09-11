// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage::{
    add_family_member, is_any_member, is_family_member, must_get_family, remove_family_member,
    save_family,
};
use crate::families::queries::get_family_by_label;
use crate::families::signature_helpers::verify_family_join_permit;
use crate::support::helpers::ensure_bonded;
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey};
use nym_contracts_common::signing::MessageSignature;

/// Creates a new MixNode family with senders node as head
pub(crate) fn try_create_family(
    deps: DepsMut,
    info: MessageInfo,
    label: String,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &info.sender)?;

    ensure_bonded(&existing_bond)?;

    let family_head = FamilyHead::new(existing_bond.identity());

    // can't overwrite existing family
    if must_get_family(&family_head, deps.storage).is_ok() {
        return Err(MixnetContractError::FamilyCanHaveOnlyOne);
    }

    // the label must be unique
    if get_family_by_label(label.clone(), deps.storage)?
        .family
        .is_some()
    {
        return Err(MixnetContractError::FamilyWithLabelExists(label));
    }

    let family = Family::new(family_head, label);
    save_family(&family, deps.storage)?;
    Ok(Response::default())
}

pub(crate) fn try_join_family(
    deps: DepsMut,
    info: MessageInfo,
    join_permit: MessageSignature,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &info.sender)?;

    ensure_bonded(&existing_bond)?;

    if family_head.identity() == existing_bond.identity() {
        return Err(MixnetContractError::CantJoinOwnFamily {
            head: family_head.identity().to_string(),
            member: existing_bond.identity().to_string(),
        });
    }

    if let Some(family) = is_any_member(deps.storage, existing_bond.identity())? {
        return Err(MixnetContractError::AlreadyMemberOfFamily(
            family.identity().to_string(),
        ));
    }

    verify_family_join_permit(
        deps.as_ref(),
        family_head.clone(),
        existing_bond.identity(),
        join_permit,
    )?;

    let family = must_get_family(&family_head, deps.storage)?;

    add_family_member(&family, deps.storage, existing_bond.identity())?;

    Ok(Response::default())
}

pub(crate) fn try_leave_family(
    deps: DepsMut,
    info: MessageInfo,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &info.sender)?;

    ensure_bonded(&existing_bond)?;

    if family_head.identity() == existing_bond.identity() {
        return Err(MixnetContractError::CantLeaveOwnFamily {
            head: family_head.identity().to_string(),
            member: existing_bond.identity().to_string(),
        });
    }

    let family = must_get_family(&family_head, deps.storage)?;
    if !is_family_member(deps.storage, &family, existing_bond.identity())? {
        return Err(MixnetContractError::NotAMember {
            head: family_head.identity().to_string(),
            member: existing_bond.identity().to_string(),
        });
    }

    remove_family_member(deps.storage, existing_bond.identity());

    Ok(Response::default())
}

pub(crate) fn try_head_kick_member(
    deps: DepsMut,
    info: MessageInfo,
    member: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let head_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &info.sender)?;

    // make sure we're still in the mixnet
    ensure_bonded(&head_bond)?;

    // make sure we're not trying to kick ourselves...
    if member == head_bond.identity() {
        return Err(MixnetContractError::CantLeaveOwnFamily {
            head: head_bond.identity().to_string(),
            member,
        });
    }

    // get the family details
    let family_head = FamilyHead::new(head_bond.identity());
    let family = must_get_family(&family_head, deps.storage)?;

    // make sure the member we're trying to kick is an actual member
    if !is_family_member(deps.storage, &family, &member)? {
        return Err(MixnetContractError::NotAMember {
            head: family_head.identity().to_string(),
            member,
        });
    }

    // finally get rid of the member
    remove_family_member(deps.storage, &member);
    Ok(Response::default())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::families::queries::get_family_by_head;
    use crate::mixnet_contract_settings::storage::minimum_mixnode_pledge;
    use crate::support::tests::fixtures;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::testing::mock_info;

    #[test]
    fn test_family_crud() {
        let mut test = TestSetup::new();
        let env = test.env();

        let head = "alice";
        let malicious_head = "timmy";
        let member = "bob";

        let minimum_pledge = minimum_mixnode_pledge(test.deps().storage).unwrap();
        let cost_params = fixtures::mix_node_cost_params_fixture();

        let (head_mixnode, head_bond_sig, head_keypair) = test.mixnode_with_signature(head, None);
        let (malicious_mixnode, malicious_bond_sig, _malicious_keypair) =
            test.mixnode_with_signature(malicious_head, None);
        let (member_mixnode, member_bond_sig, _member_keypair) =
            test.mixnode_with_signature(member, None);

        crate::mixnodes::transactions::try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            mock_info(head, &[minimum_pledge.clone()]),
            head_mixnode.clone(),
            cost_params.clone(),
            head_bond_sig,
        )
        .unwrap();

        crate::mixnodes::transactions::try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            mock_info(malicious_head, &[minimum_pledge.clone()]),
            malicious_mixnode,
            cost_params.clone(),
            malicious_bond_sig,
        )
        .unwrap();

        crate::mixnodes::transactions::try_add_mixnode(
            test.deps_mut(),
            env,
            mock_info(member, &[minimum_pledge]),
            member_mixnode.clone(),
            cost_params,
            member_bond_sig,
        )
        .unwrap();

        try_create_family(test.deps_mut(), mock_info(head, &[]), "test".to_string()).unwrap();
        let family_head = FamilyHead::new(&head_mixnode.identity_key);
        assert!(must_get_family(&family_head, test.deps().storage).is_ok());

        let nope = try_create_family(
            test.deps_mut(),
            mock_info(malicious_head, &[]),
            "test".to_string(),
        );

        match nope {
            Ok(_) => panic!("This should fail, since family with label already exists"),
            Err(e) => match e {
                MixnetContractError::FamilyWithLabelExists(label) => assert_eq!(label, "test"),
                _ => panic!("This should return FamilyWithLabelExists"),
            },
        }

        let family = get_family_by_label("test".to_string(), test.deps().storage)
            .unwrap()
            .family;
        assert!(family.is_some());
        assert_eq!(family.unwrap().head_identity(), family_head.identity());

        let family = get_family_by_head(family_head.identity(), test.deps().storage)
            .unwrap()
            .family
            .unwrap();
        assert_eq!(family.head_identity(), family_head.identity());

        let join_permit =
            test.generate_family_join_permit(&head_keypair, &member_mixnode.identity_key);

        try_join_family(
            test.deps_mut(),
            mock_info(member, &[]),
            join_permit,
            family_head.clone(),
        )
        .unwrap();

        let family = must_get_family(&family_head, test.deps().storage).unwrap();

        assert!(
            is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );

        try_leave_family(test.deps_mut(), mock_info(member, &[]), family_head.clone()).unwrap();

        let family = must_get_family(&family_head, test.deps().storage).unwrap();
        assert!(
            !is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );

        let new_join_permit =
            test.generate_family_join_permit(&head_keypair, &member_mixnode.identity_key);

        try_join_family(
            test.deps_mut(),
            mock_info(member, &[]),
            new_join_permit,
            family_head.clone(),
        )
        .unwrap();

        let family = must_get_family(&family_head, test.deps().storage).unwrap();

        assert!(
            is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );

        try_head_kick_member(
            test.deps_mut(),
            mock_info(head, &[]),
            member_mixnode.identity_key.clone(),
        )
        .unwrap();

        let family = must_get_family(&family_head, test.deps().storage).unwrap();
        assert!(
            !is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );
    }
}
