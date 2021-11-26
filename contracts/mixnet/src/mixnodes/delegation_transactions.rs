// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use config::defaults::DENOM;
use cosmwasm_std::{coins, BankMsg, Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract::IdentityKey;
use mixnet_contract::RawDelegationData;

fn validate_delegation_stake(delegation: &[Coin]) -> Result<(), ContractError> {
    // check if anything was put as delegation
    if delegation.is_empty() {
        return Err(ContractError::EmptyDelegation);
    }

    if delegation.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if delegation[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have provided a non-zero amount in the delegation
    if delegation[0].amount.is_zero() {
        return Err(ContractError::EmptyDelegation);
    }

    Ok(())
}

pub(crate) fn try_delegate_to_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    validate_delegation_stake(&info.funds)?;

    // check if the target node actually exists
    if storage::mixnodes_read(deps.storage)
        .load(mix_identity.as_bytes())
        .is_err()
    {
        return Err(ContractError::MixNodeBondNotFound {
            identity: mix_identity,
        });
    }

    // update delegation of this delegator
    storage::mix_delegations(deps.storage, &mix_identity).update::<_, ContractError>(
        info.sender.as_bytes(),
        |existing_delegation| {
            // if no delegation existed, use default, i.e. 0
            let existing_delegation_amount = existing_delegation
                .map(|existing_delegation| existing_delegation.amount)
                .unwrap_or_default();

            // the block height is reset, if it existed
            Ok(RawDelegationData::new(
                existing_delegation_amount + info.funds[0].amount,
                env.block.height,
            ))
        },
    )?;

    // update total_delegation of this node
    storage::total_delegation(deps.storage).update::<_, ContractError>(
        mix_identity.as_bytes(),
        |total_delegation| {
            // since we know that the target node exists and because the total_delegation bucket
            // entry is created whenever the node itself is added, the unwrap here is fine
            // as the entry MUST exist
            Ok(total_delegation.unwrap() + info.funds[0].amount)
        },
    )?;

    // save information about delegations of this sender
    storage::reverse_mix_delegations(deps.storage, &info.sender)
        .save(mix_identity.as_bytes(), &())?;

    Ok(Response::default())
}

pub(crate) fn try_remove_delegation_from_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    let mut delegation_bucket = storage::mix_delegations(deps.storage, &mix_identity);
    let sender_bytes = info.sender.as_bytes();

    if let Some(delegation) = delegation_bucket.may_load(sender_bytes)? {
        // remove all delegation associated with this delegator
        delegation_bucket.remove(sender_bytes);
        storage::reverse_mix_delegations(deps.storage, &info.sender)
            .remove(mix_identity.as_bytes());

        // send delegated funds back to the delegation owner
        let return_tokens = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(delegation.amount.u128(), DENOM),
        };

        // update total_delegation of this node
        storage::total_delegation(deps.storage).update::<_, ContractError>(
            mix_identity.as_bytes(),
            |total_delegation| {
                // the first unwrap is fine because the delegation information MUST exist, otherwise we would
                // have never gotten here in the first place
                // the second unwrap is also fine because we should NEVER underflow here,
                // if we do, it means we have some serious error in our logic
                Ok(total_delegation
                    .unwrap()
                    .checked_sub(delegation.amount)
                    .unwrap())
            },
        )?;

        Ok(Response::new().add_message(return_tokens))
    } else {
        Err(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
            address: info.sender,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::storage;
    use super::*;
    use crate::mixnodes::delegation_transactions::try_delegate_to_mixnode;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::coins;

    #[cfg(test)]
    mod delegation_stake_validation {
        use super::*;
        use crate::mixnodes::delegation_transactions::validate_delegation_stake;
        use cosmwasm_std::coin;
        #[test]
        fn stake_cant_be_empty() {
            assert_eq!(
                Err(ContractError::EmptyDelegation),
                validate_delegation_stake(&[])
            )
        }
        #[test]
        fn stake_must_have_single_coin_type() {
            assert_eq!(
                Err(ContractError::MultipleDenoms),
                validate_delegation_stake(&[coin(123, DENOM), coin(123, "BTC"), coin(123, "DOGE")])
            )
        }
        #[test]
        fn stake_coin_must_be_of_correct_type() {
            assert_eq!(
                Err(ContractError::WrongDenom {}),
                validate_delegation_stake(&[coin(123, "DOGE")])
            )
        }
        #[test]
        fn stake_coin_must_have_value_greater_than_zero() {
            assert_eq!(
                Err(ContractError::EmptyDelegation),
                validate_delegation_stake(&[coin(0, DENOM)])
            )
        }
        #[test]
        fn stake_can_have_any_positive_value() {
            // this might change in the future, but right now an arbitrary (positive) value can be delegated
            assert!(validate_delegation_stake(&[coin(1, DENOM)]).is_ok());
            assert!(validate_delegation_stake(&[coin(123, DENOM)]).is_ok());
            assert!(validate_delegation_stake(&[coin(10000000000, DENOM)]).is_ok());
        }
    }

    #[cfg(test)]
    mod mix_stake_delegation {
        use super::storage;
        use super::*;
        use crate::mixnodes::bonding_transactions::try_remove_mixnode;
        use crate::support::tests::test_helpers::good_mixnode_bond;
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::mock_env;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;

        #[test]
        fn fails_if_node_doesnt_exist() {
            let mut deps = test_helpers::init_contract();
            assert_eq!(
                Err(ContractError::MixNodeBondNotFound {
                    identity: "non-existent-mix-identity".into()
                }),
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    mock_env(),
                    mock_info("sender", &coins(123, DENOM)),
                    "non-existent-mix-identity".into()
                )
            );
        }

        #[test]
        fn succeeds_for_existing_node() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            let delegation = coin(123, DENOM);
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone()
            )
            .is_ok());
            assert_eq!(
                RawDelegationData::new(delegation.amount, mock_env().block.height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );
            // node's "total_delegation" is increased
            assert_eq!(
                delegation.amount,
                storage::total_delegation_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn fails_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            assert_eq!(
                Err(ContractError::MixNodeBondNotFound {
                    identity: identity.clone()
                }),
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(delegation_owner.as_str(), &coins(123, DENOM)),
                    identity
                )
            );
        }

        #[test]
        fn succeeds_if_node_rebonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation = coin(123, DENOM);
            let delegation_owner = Addr::unchecked("sender");
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone()
            )
            .is_ok());
            assert_eq!(
                RawDelegationData::new(delegation.amount, mock_env().block.height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );
            // node's "total_delegation" is increased
            assert_eq!(
                delegation.amount,
                storage::total_delegation_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn is_possible_for_an_already_delegated_node() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            let delegation1 = coin(100, DENOM);
            let delegation2 = coin(50, DENOM);
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation1.clone()]),
                identity.clone(),
            )
            .unwrap();
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation2.clone()]),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                RawDelegationData::new(
                    delegation1.amount + delegation2.amount,
                    mock_env().block.height
                ),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );
            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation1.amount + delegation2.amount,
                storage::total_delegation_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
            )
        }
        #[test]
        fn block_height_is_updated_on_new_delegation() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            let delegation = coin(100, DENOM);
            let env1 = mock_env();
            let mut env2 = mock_env();
            let initial_height = env1.block.height;
            let updated_height = initial_height + 42;
            // second env has grown in block height
            env2.block.height = updated_height;
            try_delegate_to_mixnode(
                deps.as_mut(),
                env1,
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                RawDelegationData::new(delegation.amount, initial_height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            try_delegate_to_mixnode(
                deps.as_mut(),
                env2,
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                RawDelegationData::new(delegation.amount + delegation.amount, updated_height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
        }

        #[test]
        fn block_height_is_not_updated_on_different_delegator() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner1 = Addr::unchecked("sender1");
            let delegation_owner2 = Addr::unchecked("sender2");
            let delegation1 = coin(100, DENOM);
            let delegation2 = coin(120, DENOM);
            let env1 = mock_env();
            let mut env2 = mock_env();
            let initial_height = env1.block.height;
            let second_height = initial_height + 42;
            // second env has grown in block height
            env2.block.height = second_height;
            try_delegate_to_mixnode(
                deps.as_mut(),
                env1,
                mock_info(delegation_owner1.as_str(), &[delegation1.clone()]),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                RawDelegationData::new(delegation1.amount, initial_height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner1.as_bytes())
                    .unwrap()
            );
            try_delegate_to_mixnode(
                deps.as_mut(),
                env2,
                mock_info(delegation_owner2.as_str(), &[delegation2.clone()]),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                RawDelegationData::new(delegation1.amount, initial_height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner1.as_bytes())
                    .unwrap()
            );
            assert_eq!(
                RawDelegationData::new(delegation2.amount, second_height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner2.as_bytes())
                    .unwrap()
            );
        }

        #[test]
        fn is_disallowed_for_already_delegated_node_if_it_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            assert_eq!(
                Err(ContractError::MixNodeBondNotFound {
                    identity: identity.clone()
                }),
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(delegation_owner.as_str(), &coins(50, DENOM)),
                    identity
                )
            );
        }

        #[test]
        fn is_allowed_for_multiple_nodes() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner1 = "bob";
            let mixnode_owner2 = "fred";
            let identity1 =
                test_helpers::add_mixnode(mixnode_owner1, good_mixnode_bond(), deps.as_mut());
            let identity2 =
                test_helpers::add_mixnode(mixnode_owner2, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(123, DENOM)),
                identity1.clone()
            )
            .is_ok());
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(42, DENOM)),
                identity2.clone()
            )
            .is_ok());
            assert_eq!(
                RawDelegationData::new(123u128.into(), mock_env().block.height),
                storage::mix_delegations_read(&deps.storage, &identity1)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity1.as_bytes())
                    .is_ok()
            );
            assert_eq!(
                RawDelegationData::new(42u128.into(), mock_env().block.height),
                storage::mix_delegations_read(&deps.storage, &identity2)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity2.as_bytes())
                    .is_ok()
            );
        }

        #[test]
        fn is_allowed_by_multiple_users() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation1 = coin(123, DENOM);
            let delegation2 = coin(234, DENOM);
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info("sender1", &[delegation1.clone()]),
                identity.clone()
            )
            .is_ok());
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info("sender2", &[delegation2.clone()]),
                identity.clone()
            )
            .is_ok());
            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation1.amount + delegation2.amount,
                storage::total_delegation_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
            )
        }
        #[test]
        fn delegation_is_not_removed_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            assert_eq!(
                RawDelegationData::new(100u128.into(), mock_env().block.height),
                storage::mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );
        }
    }

    #[cfg(test)]
    mod removing_mix_stake_delegation {
        use super::storage;
        use super::*;
        use crate::mixnodes::bonding_transactions::try_remove_mixnode;
        use crate::support::tests::test_helpers::good_mixnode_bond;
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::mock_env;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;
        use cosmwasm_std::Uint128;

        #[test]
        fn fails_if_delegation_never_existed() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            assert_eq!(
                Err(ContractError::NoMixnodeDelegationFound {
                    identity: identity.clone(),
                    address: delegation_owner.clone(),
                }),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    mock_info(delegation_owner.as_str(), &[]),
                    identity,
                )
            );
        }
        #[test]
        fn succeeds_if_delegation_existed() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                Ok(Response::new().add_message(BankMsg::Send {
                    to_address: delegation_owner.clone().into(),
                    amount: coins(100, DENOM),
                })),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    mock_info(delegation_owner.as_str(), &[]),
                    identity.clone(),
                )
            );
            assert!(storage::mix_delegations_read(&deps.storage, &identity)
                .may_load(delegation_owner.as_bytes())
                .unwrap()
                .is_none());
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .may_load(identity.as_bytes())
                    .unwrap()
                    .is_none()
            );
            // and total delegation is cleared
            assert_eq!(
                Uint128::zero(),
                storage::total_delegation_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
            )
        }
        #[test]
        fn succeeds_if_delegation_existed_even_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            assert_eq!(
                Ok(Response::new().add_message(BankMsg::Send {
                    to_address: delegation_owner.clone().into(),
                    amount: coins(100, DENOM),
                })),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    mock_info(delegation_owner.as_str(), &[]),
                    identity.clone(),
                )
            );
            assert!(storage::mix_delegations_read(&deps.storage, &identity)
                .may_load(delegation_owner.as_bytes())
                .unwrap()
                .is_none());
            assert!(
                storage::reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .may_load(identity.as_bytes())
                    .unwrap()
                    .is_none()
            );
        }
        #[test]
        fn total_delegation_is_preserved_if_only_some_undelegate() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity =
                test_helpers::add_mixnode(mixnode_owner, good_mixnode_bond(), deps.as_mut());
            let delegation_owner1 = Addr::unchecked("sender1");
            let delegation_owner2 = Addr::unchecked("sender2");
            let delegation1 = coin(123, DENOM);
            let delegation2 = coin(234, DENOM);
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner1.as_str(), &[delegation1.clone()]),
                identity.clone()
            )
            .is_ok());
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner2.as_str(), &[delegation2.clone()]),
                identity.clone()
            )
            .is_ok());
            // sender1 undelegates
            try_remove_delegation_from_mixnode(
                deps.as_mut(),
                mock_info(delegation_owner1.as_str(), &[]),
                identity.clone(),
            )
            .unwrap();
            // but total delegation should still equal to what sender2 sent
            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation2.amount,
                storage::total_delegation_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
            )
        }
    }

    #[cfg(test)]
    mod multi_delegations {
        use super::*;
        use crate::mixnodes::delegation_helpers;
        use crate::mixnodes::delegation_queries::tests::store_n_mix_delegations;
        use crate::support::tests::test_helpers;
        use mixnet_contract::IdentityKey;
        use mixnet_contract::RawDelegationData;

        #[test]
        fn multiple_page_delegations() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );
            let mix_bucket = storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage);
            let mix_delegations = delegation_helpers::Delegations::new(mix_bucket);
            assert_eq!(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                mix_delegations.count() as u32
            );
        }
    }
}
