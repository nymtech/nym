// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use super::storage;
use crate::error::ContractError;
use crate::mixnodes::storage as mixnodes_storage;
use crate::support::helpers::generate_storage_key;
use config::defaults::DENOM;
use cosmwasm_std::{coins, wasm_execute, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response};
use cw_storage_plus::PrimaryKey;
use mixnet_contract::Delegation;
use mixnet_contract::IdentityKey;
use vesting_contract::messages::ExecuteMsg as VestingContractExecuteMsg;

fn validate_delegation_stake(mut delegation: Vec<Coin>) -> Result<Coin, ContractError> {
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

    Ok(delegation.pop().unwrap())
}

pub(crate) fn try_delegate_to_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    let amount = validate_delegation_stake(info.funds)?;

    _try_delegate_to_mixnode(deps, env, mix_identity, info.sender.as_str(), amount, None)
}

pub(crate) fn try_delegate_to_mixnode_on_behalf(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    delegate: String,
) -> Result<Response, ContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    let amount = validate_delegation_stake(info.funds)?;

    _try_delegate_to_mixnode(
        deps,
        env,
        mix_identity,
        &delegate,
        amount,
        Some(info.sender),
    )
}

pub(crate) fn _try_delegate_to_mixnode(
    deps: DepsMut,
    env: Env,
    mix_identity: IdentityKey,
    delegate: &str,
    amount: Coin,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let delegate = deps.api.addr_validate(delegate)?;

    // check if the target node actually exists
    if mixnodes_storage::mixnodes()
        .may_load(deps.storage, &mix_identity)?
        .is_none()
    {
        return Err(ContractError::MixNodeBondNotFound {
            identity: mix_identity,
        });
    }

    let maybe_proxy_storage = generate_storage_key(&delegate, proxy.as_ref());
    let storage_key = (mix_identity.clone(), maybe_proxy_storage).joined_key();

    // update total_delegation of this node
    mixnodes_storage::TOTAL_DELEGATION.update::<_, ContractError>(
        deps.storage,
        &mix_identity,
        |total_delegation| {
            // since we know that the target node exists and because the total_delegation bucket
            // entry is created whenever the node itself is added, the unwrap here is fine
            // as the entry MUST exist
            Ok(total_delegation.unwrap() + amount.amount)
        },
    )?;

    // update [or create new] delegation of this delegator
    storage::delegations().update::<_, ContractError>(
        deps.storage,
        storage_key,
        |existing_delegation| {
            Ok(match existing_delegation {
                Some(mut existing_delegation) => {
                    existing_delegation.increment_amount(amount.amount, Some(env.block.height));
                    existing_delegation
                }
                None => Delegation::new(
                    delegate.to_owned(),
                    mix_identity,
                    amount,
                    env.block.height,
                    proxy,
                ),
            })
        },
    )?;

    Ok(Response::default())
}

pub(crate) fn try_remove_delegation_from_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    _try_remove_delegation_from_mixnode(deps, mix_identity, info.sender.as_str(), None)
}

pub(crate) fn try_remove_delegation_from_mixnode_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    mix_identity: IdentityKey,
    delegate: String,
) -> Result<Response, ContractError> {
    _try_remove_delegation_from_mixnode(deps, mix_identity, &delegate, Some(info.sender))
}

pub(crate) fn _try_remove_delegation_from_mixnode(
    deps: DepsMut,
    mix_identity: IdentityKey,
    delegate: &str,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let delegate = deps.api.addr_validate(delegate)?;
    let delegation_map = storage::delegations();
    let maybe_proxy_storage = generate_storage_key(&delegate, proxy.as_ref());
    let storage_key = (mix_identity.clone(), maybe_proxy_storage).joined_key();

    match delegation_map.may_load(deps.storage, storage_key.clone())? {
        None => Err(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
            address: delegate,
        }),
        Some(old_delegation) => {
            // remove all delegation associated with this delegator
            if proxy != old_delegation.proxy {
                return Err(ContractError::ProxyMismatch {
                    existing: old_delegation
                        .proxy
                        .map_or_else(|| "None".to_string(), |a| a.to_string()),
                    incoming: proxy.map_or_else(|| "None".to_string(), |a| a.to_string()),
                });
            }
            // remove old delegation data from the store
            // note for reviewers: I'm using `replace` as `remove` is just `may_load` followed by `replace`
            // and we've already performed `may_load` and have access to pre-existing data
            delegation_map.replace(deps.storage, storage_key, None, Some(&old_delegation))?;

            // send delegated funds back to the delegation owner
            let return_tokens = BankMsg::Send {
                to_address: proxy.as_ref().unwrap_or(&delegate).to_string(),
                amount: coins(
                    old_delegation.amount.amount.u128(),
                    old_delegation.amount.denom.clone(),
                ),
            };

            // update total_delegation of this node
            mixnodes_storage::TOTAL_DELEGATION.update::<_, ContractError>(
                deps.storage,
                &mix_identity,
                |total_delegation| {
                    // the first unwrap is fine because the delegation information MUST exist, otherwise we would
                    // have never gotten here in the first place
                    // the second unwrap is also fine because we should NEVER underflow here,
                    // if we do, it means we have some serious error in our logic
                    Ok(total_delegation
                        .unwrap()
                        .checked_sub(old_delegation.amount.amount)
                        .unwrap())
                },
            )?;

            let mut response = Response::new().add_message(return_tokens);

            if let Some(proxy) = &proxy {
                let msg = Some(VestingContractExecuteMsg::TrackUndelegation {
                    owner: delegate.as_str().to_string(),
                    mix_identity: mix_identity.clone(),
                    amount: old_delegation.amount,
                });

                let track_undelegation_msg = wasm_execute(proxy, &msg, coins(0, DENOM))?;

                response = response.add_message(track_undelegation_msg);
            }
            Ok(response)
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coins;

    use crate::support::tests::test_helpers;

    use super::storage;
    use super::*;

    #[cfg(test)]
    mod delegation_stake_validation {
        use cosmwasm_std::coin;

        use super::*;

        #[test]
        fn stake_cant_be_empty() {
            assert_eq!(
                Err(ContractError::EmptyDelegation),
                validate_delegation_stake(vec![])
            )
        }

        #[test]
        fn stake_must_have_single_coin_type() {
            assert_eq!(
                Err(ContractError::MultipleDenoms),
                validate_delegation_stake(vec![
                    coin(123, DENOM),
                    coin(123, "BTC"),
                    coin(123, "DOGE")
                ])
            )
        }

        #[test]
        fn stake_coin_must_be_of_correct_type() {
            assert_eq!(
                Err(ContractError::WrongDenom {}),
                validate_delegation_stake(coins(123, "DOGE"))
            )
        }

        #[test]
        fn stake_coin_must_have_value_greater_than_zero() {
            assert_eq!(
                Err(ContractError::EmptyDelegation),
                validate_delegation_stake(coins(0, DENOM))
            )
        }

        #[test]
        fn stake_can_have_any_positive_value() {
            // this might change in the future, but right now an arbitrary (positive) value can be delegated
            assert!(validate_delegation_stake(coins(1, DENOM)).is_ok());
            assert!(validate_delegation_stake(coins(123, DENOM)).is_ok());
            assert!(validate_delegation_stake(coins(10000000000, DENOM)).is_ok());
        }
    }

    #[cfg(test)]
    mod mix_stake_delegation {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
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
                    "non-existent-mix-identity".into(),
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
                identity.clone(),
            )
            .is_ok());

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                delegation.clone(),
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(&deps.storage, &identity, delegation_owner).unwrap()
            );

            // node's "total_delegation" is increased
            assert_eq!(
                delegation.amount,
                mixnodes_storage::TOTAL_DELEGATION
                    .load(&deps.storage, &identity)
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
                    identity,
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
                identity.clone(),
            )
            .is_ok());

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                delegation.clone(),
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(&deps.storage, &identity, delegation_owner).unwrap()
            );

            // node's "total_delegation" is increased
            assert_eq!(
                delegation.amount,
                mixnodes_storage::TOTAL_DELEGATION
                    .load(&deps.storage, &identity)
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

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                coin(delegation1.amount.u128() + delegation2.amount.u128(), DENOM),
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(&deps.storage, &identity, delegation_owner).unwrap()
            );

            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation1.amount + delegation2.amount,
                mixnodes_storage::TOTAL_DELEGATION
                    .load(&deps.storage, &identity)
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
                initial_height,
                test_helpers::read_delegation(&deps.storage, &identity, &delegation_owner)
                    .unwrap()
                    .block_height
            );
            try_delegate_to_mixnode(
                deps.as_mut(),
                env2,
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone(),
            )
            .unwrap();

            let updated =
                test_helpers::read_delegation(&deps.storage, &identity, &delegation_owner).unwrap();

            assert_eq!(delegation.amount + delegation.amount, updated.amount.amount);
            assert_eq!(updated_height, updated.block_height);
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
                initial_height,
                test_helpers::read_delegation(&deps.storage, &identity, &delegation_owner1)
                    .unwrap()
                    .block_height
            );
            try_delegate_to_mixnode(
                deps.as_mut(),
                env2,
                mock_info(delegation_owner2.as_str(), &[delegation2.clone()]),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                initial_height,
                test_helpers::read_delegation(&deps.storage, &identity, &delegation_owner1)
                    .unwrap()
                    .block_height
            );
            assert_eq!(
                second_height,
                test_helpers::read_delegation(&deps.storage, identity, &delegation_owner2)
                    .unwrap()
                    .block_height
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
                    identity,
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
                identity1.clone(),
            )
            .is_ok());
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(42, DENOM)),
                identity2.clone(),
            )
            .is_ok());

            let expected1 = Delegation::new(
                delegation_owner.clone(),
                identity1.clone(),
                coin(123, DENOM),
                mock_env().block.height,
                None,
            );

            let expected2 = Delegation::new(
                delegation_owner.clone(),
                identity2.clone(),
                coin(42, DENOM),
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected1,
                test_helpers::read_delegation(&deps.storage, identity1, &delegation_owner).unwrap()
            );
            assert_eq!(
                expected2,
                test_helpers::read_delegation(&deps.storage, identity2, &delegation_owner).unwrap()
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
                identity.clone(),
            )
            .is_ok());
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info("sender2", &[delegation2.clone()]),
                identity.clone(),
            )
            .is_ok());
            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation1.amount + delegation2.amount,
                mixnodes_storage::TOTAL_DELEGATION
                    .load(&deps.storage, &identity)
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
            let delegation_amount = coin(100, DENOM);
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &vec![delegation_amount.clone()]),
                identity.clone(),
            )
            .unwrap();
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                delegation_amount,
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(&deps.storage, identity, delegation_owner).unwrap()
            )
        }
    }

    #[cfg(test)]
    mod removing_mix_stake_delegation {
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::mock_env;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;
        use cosmwasm_std::Uint128;

        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::test_helpers::good_mixnode_bond;

        use super::storage;
        use super::*;

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
            assert!(storage::delegations()
                .may_load(
                    &deps.storage,
                    (identity.clone(), delegation_owner).joined_key(),
                )
                .unwrap()
                .is_none());

            // and total delegation is cleared
            assert_eq!(
                Uint128::zero(),
                mixnodes_storage::TOTAL_DELEGATION
                    .load(&deps.storage, &identity)
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

            assert!(
                test_helpers::read_delegation(&deps.storage, identity, delegation_owner).is_none()
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
                identity.clone(),
            )
            .is_ok());
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner2.as_str(), &[delegation2.clone()]),
                identity.clone(),
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
                mixnodes_storage::TOTAL_DELEGATION
                    .load(&deps.storage, &identity)
                    .unwrap()
            )
        }
    }

    // #[cfg(test)]
    // mod multi_delegations {
    //     use super::*;
    //     use crate::delegations::helpers;
    //     use crate::delegations::queries::tests::store_n_mix_delegations;
    //     use crate::support::tests::test_helpers;
    //     use mixnet_contract::IdentityKey;
    //     use mixnet_contract::RawDelegationData;
    //
    //     #[test]
    //     fn multiple_page_delegations() {
    //         let mut deps = test_helpers::init_contract();
    //         let node_identity: IdentityKey = "foo".into();
    //         store_n_mix_delegations(
    //             storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
    //             &mut deps.storage,
    //             &node_identity,
    //         );
    //         let mix_bucket = storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage);
    //         let mix_delegations = helpers::Delegations::new(mix_bucket);
    //         assert_eq!(
    //             storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
    //             mix_delegations.count() as u32
    //         );
    //     }
    // }
}
