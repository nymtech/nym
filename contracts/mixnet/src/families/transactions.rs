// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage::{
    add_family_member, is_any_member, is_family_member, must_get_family, remove_family_member,
    save_family,
};
use crate::families::queries::get_family_by_label;
use crate::families::signature_helpers::verify_family_join_permit;
use crate::support::helpers::{ensure_bonded, ensure_sent_by_vesting_contract};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey};
use nym_contracts_common::signing::MessageSignature;

/// Creates a new MixNode family with senders node as head
pub fn try_create_family(
    deps: DepsMut,
    info: MessageInfo,
    label: String,
) -> Result<Response, MixnetContractError> {
    _try_create_family(deps, &info.sender, label, None)
}

pub fn try_create_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    owner_address: String,
    label: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let owner_address = deps.api.addr_validate(&owner_address)?;
    _try_create_family(deps, &owner_address, label, Some(info.sender))
}

fn _try_create_family(
    deps: DepsMut,
    owner: &Addr,
    label: String,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, owner)?;

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

    let family = Family::new(family_head, proxy, label);
    save_family(&family, deps.storage)?;
    Ok(Response::default())
}

pub fn try_join_family(
    deps: DepsMut,
    info: MessageInfo,
    join_permit: MessageSignature,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    _try_join_family(deps, &info.sender, join_permit, family_head, None)
}

pub fn try_join_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    member_address: String,
    join_permit: MessageSignature,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let member_address = deps.api.addr_validate(&member_address)?;
    let proxy = Some(info.sender);
    _try_join_family(deps, &member_address, join_permit, family_head, proxy)
}

fn _try_join_family(
    deps: DepsMut,
    owner: &Addr,
    join_permit: MessageSignature,
    family_head: FamilyHead,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, owner)?;

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
        proxy,
        existing_bond.identity(),
        join_permit,
    )?;

    let family = must_get_family(&family_head, deps.storage)?;

    add_family_member(&family, deps.storage, existing_bond.identity())?;

    Ok(Response::default())
}

pub fn try_leave_family(
    deps: DepsMut,
    info: MessageInfo,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    _try_leave_family(deps, &info.sender, family_head)
}

pub fn try_leave_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    member_address: String,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let member_address = deps.api.addr_validate(&member_address)?;
    _try_leave_family(deps, &member_address, family_head)
}

fn _try_leave_family(
    deps: DepsMut,
    owner: &Addr,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, owner)?;

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

pub fn try_head_kick_member(
    deps: DepsMut,
    info: MessageInfo,
    member: IdentityKey,
) -> Result<Response, MixnetContractError> {
    _try_head_kick_member(deps, &info.sender, member)
}

pub fn try_head_kick_member_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    head_address: String,
    member: IdentityKey,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let head_address = deps.api.addr_validate(&head_address)?;
    _try_head_kick_member(deps, &head_address, member)
}

fn _try_head_kick_member(
    deps: DepsMut,
    owner: &Addr,
    member: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let head_bond = crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, owner)?;

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
            test.generate_family_join_permit(&head_keypair, &member_mixnode.identity_key, false);

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
            test.generate_family_join_permit(&head_keypair, &member_mixnode.identity_key, false);

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

    #[cfg(test)]
    mod creating_family {
        use super::*;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";

            test.add_dummy_mixnode(head, None);

            let res = try_create_family_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                head.to_string(),
                "label".to_string(),
            )
            .unwrap_err();

            assert_eq!(
                res,
                MixnetContractError::SenderIsNotVestingContract {
                    received: illegal_proxy,
                    vesting_contract
                }
            )
        }
    }

    #[cfg(test)]
    mod joining_family {
        use super::*;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";
            let label = "family";
            let new_member = "vin-diesel";

            let (_, head_keys) = test.create_dummy_mixnode_with_new_family(head, label);
            let (_, member_keys) = test.add_dummy_mixnode_with_keypair(new_member, None);

            let join_permit = test.generate_family_join_permit(
                &head_keys,
                &member_keys.public_key().to_base58_string(),
                false,
            );

            let head_identity = head_keys.public_key().to_base58_string();
            let family_head = FamilyHead::new(head_identity);
            let res = try_join_family_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                new_member.to_string(),
                join_permit,
                family_head,
            )
            .unwrap_err();

            assert_eq!(
                res,
                MixnetContractError::SenderIsNotVestingContract {
                    received: illegal_proxy,
                    vesting_contract
                }
            )
        }
    }

    #[cfg(test)]
    mod leaving_family {
        use super::*;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";
            let label = "family";
            let new_member = "vin-diesel";

            let (_, head_keys) = test.create_dummy_mixnode_with_new_family(head, label);
            let (_, member_keys) = test.add_dummy_mixnode_with_keypair(new_member, None);

            let join_permit = test.generate_family_join_permit(
                &head_keys,
                &member_keys.public_key().to_base58_string(),
                true,
            );

            let head_identity = head_keys.public_key().to_base58_string();
            let family_head = FamilyHead::new(head_identity);
            try_join_family_on_behalf(
                test.deps_mut(),
                mock_info(vesting_contract.as_ref(), &[]),
                new_member.to_string(),
                join_permit,
                family_head.clone(),
            )
            .unwrap();

            let res = try_leave_family_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                new_member.to_string(),
                family_head,
            )
            .unwrap_err();

            assert_eq!(
                res,
                MixnetContractError::SenderIsNotVestingContract {
                    received: illegal_proxy,
                    vesting_contract
                }
            )
        }
    }

    #[cfg(test)]
    mod kicking_family_member {
        use super::*;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";
            let label = "family";
            let new_member = "vin-diesel";

            let (_, head_keys) = test.create_dummy_mixnode_with_new_family(head, label);
            let (_, member_keys) = test.add_dummy_mixnode_with_keypair(new_member, None);

            let join_permit = test.generate_family_join_permit(
                &head_keys,
                &member_keys.public_key().to_base58_string(),
                true,
            );

            let head_identity = head_keys.public_key().to_base58_string();
            let family_head = FamilyHead::new(head_identity);

            try_join_family_on_behalf(
                test.deps_mut(),
                mock_info(vesting_contract.as_ref(), &[]),
                new_member.to_string(),
                join_permit,
                family_head,
            )
            .unwrap();

            let res = try_head_kick_member_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                head.to_string(),
                member_keys.public_key().to_base58_string(),
            )
            .unwrap_err();

            assert_eq!(
                res,
                MixnetContractError::SenderIsNotVestingContract {
                    received: illegal_proxy,
                    vesting_contract
                }
            )
        }
    }
}
