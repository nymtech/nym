use crate::support::helpers::{
    ensure_bonded, ensure_sent_by_vesting_contract, validate_family_signature,
    validate_node_identity_signature,
};

use crate::families::signature_helpers::{
    verify_family_creation_signature, verify_family_join_permit,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey, IdentityKeyRef};
use nym_contracts_common::signing::MessageSignature;

use super::storage::{
    add_family_member, create_family, get_family, is_any_member, is_family_member,
    remove_family_member,
};

/// Creates a new MixNode family with senders node as head
pub fn try_create_family(
    deps: DepsMut,
    info: MessageInfo,
    owner_signature: MessageSignature,
    label: String,
) -> Result<Response, MixnetContractError> {
    _try_create_family(deps, &info.sender, owner_signature, label, None)
}

pub fn try_create_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    owner_address: String,
    owner_signature: MessageSignature,
    label: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let owner_address = deps.api.addr_validate(&owner_address)?;
    _try_create_family(
        deps,
        &owner_address,
        owner_signature,
        label,
        Some(info.sender),
    )
}

fn _try_create_family(
    deps: DepsMut,
    owner: &Addr,
    owner_signature: MessageSignature,
    label: String,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    ensure_bonded(&existing_bond)?;

    verify_family_creation_signature(
        deps.as_ref(),
        owner.clone(),
        proxy.clone(),
        label.clone(),
        existing_bond.identity(),
        owner_signature,
    )?;

    let family_head = FamilyHead::new(existing_bond.identity());

    if let Ok(_family) = get_family(&family_head, deps.storage) {
        return Err(MixnetContractError::FamilyCanHaveOnlyOne);
    }

    let family = Family::new(family_head, proxy, label);
    create_family(&family, deps.storage)?;
    Ok(Response::default())
}

pub fn try_join_family(
    deps: DepsMut,
    info: MessageInfo,
    join_permit: MessageSignature,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let family_head = FamilyHead::new(&family_head);
    _try_join_family(deps, &info.sender, join_permit, family_head, None)
}

pub fn try_join_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    member_address: String,
    join_permit: MessageSignature,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let member_address = deps.api.addr_validate(&member_address)?;
    let family_head = FamilyHead::new(&family_head);
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
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

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

    let family = get_family(&family_head, deps.storage)?;

    add_family_member(&family, deps.storage, existing_bond.identity())?;

    Ok(Response::default())
}

pub fn try_leave_family(
    deps: DepsMut,
    info: MessageInfo,
    signature: String,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let family_head = FamilyHead::new(&family_head);
    _try_leave_family(deps, &info.sender, signature, family_head)
}

pub fn try_leave_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    member_address: String,
    node_family_signature: String,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let family_head = FamilyHead::new(&family_head);
    let member_address = deps.api.addr_validate(&member_address)?;
    _try_leave_family(deps, &member_address, node_family_signature, family_head)
}

fn _try_leave_family(
    deps: DepsMut,
    owner: &Addr,
    node_family_signature: String,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    let existing_bond =
        crate::mixnodes::helpers::must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    ensure_bonded(&existing_bond)?;

    if family_head.identity() == existing_bond.identity() {
        return Err(MixnetContractError::CantLeaveOwnFamily {
            head: family_head.identity().to_string(),
            member: existing_bond.identity().to_string(),
        });
    }

    let family = get_family(&family_head, deps.storage)?;
    if !is_family_member(deps.storage, &family, existing_bond.identity())? {
        return Err(MixnetContractError::NotAMember {
            head: family_head.identity().to_string(),
            member: existing_bond.identity().to_string(),
        });
    }

    validate_node_identity_signature(
        deps.as_ref(),
        owner,
        &node_family_signature,
        existing_bond.identity(),
    )?;

    remove_family_member(deps.storage, existing_bond.identity());

    Ok(Response::default())
}

pub fn try_head_kick_member(
    deps: DepsMut,
    info: MessageInfo,
    owner_signature: String,
    member: IdentityKeyRef,
) -> Result<Response, MixnetContractError> {
    _try_head_kick_member(deps, &info.sender, owner_signature, member)
}

pub fn try_head_kick_member_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    head_address: String,
    owner_signature: String,
    member: IdentityKeyRef,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let head_address = deps.api.addr_validate(&head_address)?;
    _try_head_kick_member(deps, &head_address, owner_signature, member)
}

#[allow(unused_variables)]
fn _try_head_kick_member(
    deps: DepsMut,
    owner: &Addr,
    owner_signature: String,
    member: IdentityKeyRef<'_>,
) -> Result<Response, MixnetContractError> {
    Err(MixnetContractError::NotImplemented)
    // let existing_bond = crate::mixnodes::storage::mixnode_bonds()
    //     .idx
    //     .owner
    //     .item(deps.storage, owner.clone())?
    //     .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
    //         owner: owner.clone(),
    //     })?
    //     .1;

    // ensure_bonded(&existing_bond)?;

    // validate_node_identity_signature(
    //     deps.as_ref(),
    //     owner,
    //     &owner_signature,
    //     existing_bond.identity(),
    // )?;

    // let family_head = FamilyHead::new(existing_bond.identity());
    // let family = get_family(&family_head, deps.storage)?;
    // if !is_family_member(deps.storage, &family, member)? {
    //     return Err(MixnetContractError::NotAMember {
    //         head: family_head.identity().to_string(),
    //         member: existing_bond.identity().to_string(),
    //     });
    // }

    // remove_family_member(deps.storage, member);
    // Ok(Response::default())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::families::queries::{get_family_by_head, get_family_by_label};
    use crate::families::storage::is_family_member;
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
        let (malicious_mixnode, malicious_bond_sig, malicious_keypair) =
            test.mixnode_with_signature(malicious_head, None);
        let (member_mixnode, member_bond_sig, member_keypair) =
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

        let old_style_head_sig = head_keypair.private_key().sign_text(head);
        let old_style_malicious_head_sig =
            malicious_keypair.private_key().sign_text(malicious_head);
        let old_style_member_sig = member_keypair.private_key().sign_text(member);

        try_create_family(
            test.deps_mut(),
            mock_info(head, &[]),
            old_style_head_sig,
            "test".to_string(),
        )
        .unwrap();
        let family_head = FamilyHead::new(&head_mixnode.identity_key);
        assert!(get_family(&family_head, test.deps().storage).is_ok());

        let nope = try_create_family(
            test.deps_mut(),
            mock_info(malicious_head, &[]),
            old_style_malicious_head_sig,
            "test".to_string(),
        );

        match nope {
            Ok(_) => panic!("This should fail, since family with label already exists"),
            Err(e) => match e {
                MixnetContractError::FamilyWithLabelExists(label) => assert_eq!(label, "test"),
                _ => panic!("This should return FamilyWithLabelExists"),
            },
        }

        let family = get_family_by_label("test".to_string(), test.deps().storage).unwrap();
        assert!(family.is_some());
        assert_eq!(family.unwrap().head_identity(), family_head.identity());

        let family = get_family_by_head(family_head.identity(), test.deps().storage).unwrap();
        assert_eq!(family.head_identity(), family_head.identity());

        let join_signature = head_keypair
            .private_key()
            .sign(member_mixnode.identity_key.as_bytes())
            .to_base58_string();

        try_join_family(
            test.deps_mut(),
            mock_info(member, &[]),
            Some(old_style_member_sig.clone()),
            join_signature.clone(),
            head_mixnode.identity_key.clone(),
        )
        .unwrap();

        let family = get_family(&family_head, test.deps().storage).unwrap();

        assert!(
            is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );

        try_leave_family(
            test.deps_mut(),
            mock_info(member, &[]),
            old_style_member_sig.clone(),
            head_mixnode.identity_key.clone(),
        )
        .unwrap();

        let family = get_family(&family_head, test.deps().storage).unwrap();
        assert!(
            !is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );

        try_join_family(
            test.deps_mut(),
            mock_info(member, &[]),
            Some(old_style_member_sig),
            join_signature,
            head_mixnode.identity_key,
        )
        .unwrap();

        let family = get_family(&family_head, test.deps().storage).unwrap();

        assert!(
            is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap()
        );

        // try_head_kick_member(
        //     deps.as_mut(),
        //     mock_info(&head, &[]),
        //     head_sig.clone(),
        //     &member_mixnode.identity_key.clone(),
        // )
        // .unwrap();

        // let family = get_family(&family_head, test.deps().storage).unwrap();
        // assert!(!is_family_member(test.deps().storage, &family, &member_mixnode.identity_key).unwrap());
    }

    #[cfg(test)]
    mod creating_family {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";

            let (_, keypair) = test.add_dummy_mixnode_with_proxy_and_keypair(head, None);
            let sig = keypair.private_key().sign_text(head);

            let res = try_create_family_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                head.to_string(),
                sig,
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
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";
            let label = "family";
            let new_member = "vin-diesel";

            let (_, head_keys) = test.create_dummy_mixnode_with_new_family(head, label);
            let (_, member_keys) = test.add_dummy_mixnode_with_proxy_and_keypair(new_member, None);

            // TODO: those signatures are WRONG and have to be c hanged
            let join_signature = head_keys
                .private_key()
                .sign_text(&member_keys.public_key().to_base58_string());

            let member_sig = member_keys.private_key().sign_text(new_member);

            let head_identity = head_keys.public_key().to_base58_string();
            let res = try_join_family_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                new_member.to_string(),
                Some(member_sig),
                join_signature,
                head_identity,
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
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";
            let label = "family";
            let new_member = "vin-diesel";

            let (_, head_keys) = test.create_dummy_mixnode_with_new_family(head, label);
            let (_, member_keys) = test.add_dummy_mixnode_with_proxy_and_keypair(new_member, None);

            // TODO: those signatures are WRONG and have to be changed
            let join_signature = head_keys
                .private_key()
                .sign_text(&member_keys.public_key().to_base58_string());

            let member_sig = member_keys.private_key().sign_text(new_member);

            let head_identity = head_keys.public_key().to_base58_string();
            try_join_family_on_behalf(
                test.deps_mut(),
                mock_info(vesting_contract.as_ref(), &[]),
                new_member.to_string(),
                Some(member_sig.clone()),
                join_signature,
                head_identity,
            )
            .unwrap();

            let leave_signature = member_sig;
            let res = try_leave_family_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                new_member.to_string(),
                leave_signature,
                head_keys.public_key().to_base58_string(),
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
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let head = "alice";
            let label = "family";
            let new_member = "vin-diesel";

            let (_, head_keys) = test.create_dummy_mixnode_with_new_family(head, label);
            let (_, member_keys) = test.add_dummy_mixnode_with_proxy_and_keypair(new_member, None);

            // TODO: those signatures are WRONG and have to be c hanged
            let join_signature = head_keys
                .private_key()
                .sign_text(&member_keys.public_key().to_base58_string());

            let member_sig = member_keys.private_key().sign_text(new_member);

            let head_identity = head_keys.public_key().to_base58_string();
            try_join_family_on_behalf(
                test.deps_mut(),
                mock_info(vesting_contract.as_ref(), &[]),
                new_member.to_string(),
                Some(member_sig),
                join_signature,
                head_identity,
            )
            .unwrap();

            // TODO: a completely wrong signature is being used
            let res = try_head_kick_member_on_behalf(
                test.deps_mut(),
                mock_info(illegal_proxy.as_ref(), &[]),
                head.to_string(),
                head_keys.private_key().sign_text(head),
                &member_keys.public_key().to_base58_string(),
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
