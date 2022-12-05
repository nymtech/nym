use crate::support::helpers::{
    ensure_bonded, validate_family_signature, validate_node_identity_signature,
};

use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey, IdentityKeyRef};

use super::storage::{
    add_family_member, create_family, get_family, is_any_member, is_family_member,
    remove_family_member,
};

/// Creates a new MixNode family with senders node as head
pub fn try_create_family(
    deps: DepsMut,
    info: MessageInfo,
    owner_signature: String,
    label: &str,
) -> Result<Response, MixnetContractError> {
    _try_create_family(deps, &info.sender, owner_signature, label, None)
}

pub fn try_create_family_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    owner_address: String,
    owner_signature: String,
    label: &str,
) -> Result<Response, MixnetContractError> {
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
    owner_signature: String,
    label: &str,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond = crate::mixnodes::storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        })?
        .1;

    ensure_bonded(&existing_bond)?;

    validate_node_identity_signature(
        deps.as_ref(),
        owner,
        &owner_signature,
        existing_bond.identity(),
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
    signature: String,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let family_head = FamilyHead::new(&family_head);
    _try_join_family(deps, &info.sender, signature, family_head)
}

pub fn try_join_family_on_behalf(
    deps: DepsMut,
    _info: MessageInfo,
    member_address: String,
    signature: String,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let member_address = deps.api.addr_validate(&member_address)?;
    let family_head = FamilyHead::new(&family_head);
    _try_join_family(deps, &member_address, signature, family_head)
}

fn _try_join_family(
    deps: DepsMut,
    owner: &Addr,
    signature: String,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    let existing_bond = crate::mixnodes::storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        })?
        .1;

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

    validate_family_signature(
        deps.as_ref(),
        existing_bond.identity(),
        &signature,
        family_head.identity(),
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
    _info: MessageInfo,
    member_address: String,
    signature: String,
    family_head: IdentityKey,
) -> Result<Response, MixnetContractError> {
    let family_head = FamilyHead::new(&family_head);
    let member_address = deps.api.addr_validate(&member_address)?;
    _try_leave_family(deps, &member_address, signature, family_head)
}

fn _try_leave_family(
    deps: DepsMut,
    owner: &Addr,
    signature: String,
    family_head: FamilyHead,
) -> Result<Response, MixnetContractError> {
    let existing_bond = crate::mixnodes::storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        })?
        .1;

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

    validate_family_signature(
        deps.as_ref(),
        existing_bond.identity(),
        &signature,
        family_head.identity(),
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
    _info: MessageInfo,
    head_address: String,
    owner_signature: String,
    member: IdentityKeyRef,
) -> Result<Response, MixnetContractError> {
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
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn test_family_crud() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let head = "alice";
        let malicious_head = "timmy";

        let minimum_pledge = minimum_mixnode_pledge(deps.as_ref().storage).unwrap();

        let (head_mixnode, head_sig, head_keypair) =
            test_helpers::mixnode_with_signature(&mut rng, head);

        let (malicious_mixnode, malicious_sig, _malicious_keypair) =
            test_helpers::mixnode_with_signature(&mut rng, malicious_head);

        let cost_params = fixtures::mix_node_cost_params_fixture();

        let member = "bob";
        let (member_mixnode, member_sig, _) =
            test_helpers::mixnode_with_signature(&mut rng, member);

        // we are informed that we didn't send enough funds
        crate::mixnodes::transactions::try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(head, &[minimum_pledge.clone()]),
            head_mixnode.clone(),
            cost_params.clone(),
            head_sig.clone(),
        )
        .unwrap();

        crate::mixnodes::transactions::try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(malicious_head, &[minimum_pledge.clone()]),
            malicious_mixnode.clone(),
            cost_params.clone(),
            malicious_sig.clone(),
        )
        .unwrap();

        crate::mixnodes::transactions::try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(member, &[minimum_pledge.clone()]),
            member_mixnode.clone(),
            cost_params.clone(),
            member_sig.clone(),
        )
        .unwrap();

        try_create_family(
            deps.as_mut(),
            mock_info(head.clone(), &[]),
            head_sig.clone(),
            "test",
        )
        .unwrap();
        let family_head = FamilyHead::new(&head_mixnode.identity_key);
        assert!(get_family(&family_head, &deps.storage).is_ok());

        let nope = try_create_family(
            deps.as_mut(),
            mock_info(malicious_head.clone(), &[]),
            malicious_sig.clone(),
            "test",
        );

        match nope {
            Ok(_) => panic!("This should fail, since family with label already exists"),
            Err(e) => match e {
                MixnetContractError::FamilyWithLabelExists(label) => assert_eq!(label, "test"),
                _ => panic!("This should return FamilyWithLabelExists"),
            },
        }

        let family = get_family_by_label("test", &deps.storage).unwrap();
        assert!(family.is_some());
        assert_eq!(family.unwrap().head_identity(), family_head.identity());

        let family = get_family_by_head(family_head.identity(), &deps.storage).unwrap();
        assert_eq!(family.head_identity(), family_head.identity());

        let join_signature = head_keypair
            .private_key()
            .sign(member_mixnode.identity_key.as_bytes())
            .to_base58_string();

        try_join_family(
            deps.as_mut(),
            mock_info(member, &[]),
            join_signature.clone(),
            head_mixnode.identity_key.clone(),
        )
        .unwrap();

        let family = get_family(&family_head, &deps.storage).unwrap();

        assert!(is_family_member(&deps.storage, &family, &member_mixnode.identity_key).unwrap());

        try_leave_family(
            deps.as_mut(),
            mock_info(member, &[]),
            join_signature.clone(),
            head_mixnode.identity_key.clone(),
        )
        .unwrap();

        let family = get_family(&family_head, &deps.storage).unwrap();
        assert!(!is_family_member(&deps.storage, &family, &member_mixnode.identity_key).unwrap());

        try_join_family(
            deps.as_mut(),
            mock_info(member, &[]),
            join_signature.clone(),
            head_mixnode.identity_key.clone(),
        )
        .unwrap();

        let family = get_family(&family_head, &deps.storage).unwrap();

        assert!(is_family_member(&deps.storage, &family, &member_mixnode.identity_key).unwrap());

        // try_head_kick_member(
        //     deps.as_mut(),
        //     mock_info(&head, &[]),
        //     head_sig.clone(),
        //     &member_mixnode.identity_key.clone(),
        // )
        // .unwrap();

        // let family = get_family(&family_head, &deps.storage).unwrap();
        // assert!(!is_family_member(&deps.storage, &family, &member_mixnode.identity_key).unwrap());
    }
}
