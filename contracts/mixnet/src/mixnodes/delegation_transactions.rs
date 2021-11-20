use super::delegation_helpers;
use super::storage;
use crate::error::ContractError;

use config::defaults::DENOM;
use cosmwasm_std::{coins, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, StdResult};
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::IdentityKey;
use mixnet_contract::RawDelegationData;

pub(crate) const OLD_DELEGATIONS_CHUNK_SIZE: usize = 500;

pub fn total_delegations(delegations_bucket: ReadonlyBucket<RawDelegationData>) -> StdResult<Coin> {
    Ok(Coin::new(
        delegation_helpers::Delegations::new(delegations_bucket)
            .fold(0, |acc, x| acc + x.delegation_data.amount.u128()),
        DENOM,
    ))
}

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
    let mut current_bond = match storage::mixnodes_read(deps.storage).load(mix_identity.as_bytes())
    {
        Ok(bond) => bond,
        Err(_) => {
            return Err(ContractError::MixNodeBondNotFound {
                identity: mix_identity,
            });
        }
    };

    let amount = info.funds[0].amount;

    // update total_delegation of this node
    current_bond.total_delegation.amount += info.funds[0].amount;
    storage::mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;

    let mut delegation_bucket = storage::mix_delegations(deps.storage, &mix_identity);
    let sender_bytes = info.sender.as_bytes();

    // write the delegation
    let new_amount = match delegation_bucket.may_load(sender_bytes)? {
        Some(existing_delegation) => existing_delegation.amount + amount,
        None => amount,
    };
    // the block height is reset, if it existed
    let new_delegation = RawDelegationData::new(new_amount, env.block.height);
    delegation_bucket.save(sender_bytes, &new_delegation)?;

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
    match delegation_bucket.may_load(sender_bytes)? {
        Some(delegation) => {
            // remove delegation from the buckets
            delegation_bucket.remove(sender_bytes);
            storage::reverse_mix_delegations(deps.storage, &info.sender)
                .remove(mix_identity.as_bytes());

            // send delegated funds back to the delegation owner
            let messages = vec![BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: coins(delegation.amount.u128(), DENOM),
            }
            .into()];

            // update total_delegation of this node
            let mut mixnodes_bucket = storage::mixnodes(deps.storage);
            // in some rare cases the mixnode bond might no longer exist as the node unbonded
            // before delegation was removed. that is fine
            if let Some(mut existing_bond) = mixnodes_bucket.may_load(mix_identity.as_bytes())? {
                // we should NEVER underflow here, if we do, it means we have some serious error in our logic
                existing_bond.total_delegation.amount = existing_bond
                    .total_delegation
                    .amount
                    .checked_sub(delegation.amount)
                    .unwrap();
                mixnodes_bucket.save(mix_identity.as_bytes(), &existing_bond)?;
            }

            Ok(Response {
                submessages: Vec::new(),
                messages,
                attributes: Vec::new(),
                data: None,
            })
        }
        None => Err(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
            address: info.sender,
        }),
    }
}
#[cfg(test)]
mod tests {
    use super::storage;
    use super::*;
    use crate::mixnodes::delegation_transactions::try_delegate_to_mixnode;
    use crate::rewards::helpers as rewards_helpers;
    use crate::rewards::transactions::MINIMUM_BLOCK_AGE_FOR_REWARDING;
    use crate::rewards::transactions::{
        try_begin_mixnode_rewarding, try_finish_mixnode_rewarding, try_reward_mixnode,
    };
    use crate::support::tests::test_helpers;
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coins, Uint128};

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
        use crate::support::tests::test_helpers::add_mixnode;
        #[cfg(test)]
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
                delegation,
                storage::mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
            )
        }
        #[test]
        fn fails_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
            add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
                delegation,
                storage::mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
            )
        }
        #[test]
        fn is_possible_for_an_already_delegated_node() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
                storage::mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
                    .amount
            )
        }
        #[test]
        fn block_height_is_updated_on_new_delegation() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
            let identity1 = add_mixnode(mixnode_owner1, good_mixnode_bond(), &mut deps);
            let identity2 = add_mixnode(mixnode_owner2, good_mixnode_bond(), &mut deps);
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
                storage::mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
                    .amount
            )
        }
        #[test]
        fn delegation_is_not_removed_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
        use crate::support::tests::test_helpers::add_mixnode;
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();
            assert_eq!(
                Ok(Response {
                    submessages: vec![],
                    messages: vec![BankMsg::Send {
                        to_address: delegation_owner.clone().into(),
                        amount: coins(100, DENOM),
                    }
                    .into()],
                    attributes: Vec::new(),
                    data: None,
                }),
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
                storage::mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
                    .amount
            )
        }
        #[test]
        fn succeeds_if_delegation_existed_even_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
                Ok(Response {
                    submessages: vec![],
                    messages: vec![BankMsg::Send {
                        to_address: delegation_owner.clone().into(),
                        amount: coins(100, DENOM),
                    }
                    .into()],
                    attributes: Vec::new(),
                    data: None,
                }),
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
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
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
                delegation2,
                storage::mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
            )
        }
    }
    #[test]
    fn delegators_on_mix_node_reward_rate() {
        use crate::mixnet_params::storage as mixnet_params_storage;

        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::contract_settings_read(deps.as_mut().storage)
            .load()
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let initial_mix_bond = 100_000000;
        let initial_delegation1 = 50000; // will see single digits rewards
        let initial_delegation2 = 100; // won't see any rewards due to such a small delegation
        let initial_delegation3 = 100000_000000; // will see big proper rewards
        let node_owner = "node-owner";
        let identity =
            test_helpers::add_mixnode(node_owner, test_helpers::good_mixnode_bond(), &mut deps);
        storage::mix_delegations(&mut deps.storage, &identity)
            .save(
                b"delegator1",
                &RawDelegationData::new(initial_delegation1.into(), env.block.height),
            )
            .unwrap();
        storage::mix_delegations(&mut deps.storage, &identity)
            .save(
                b"delegator2",
                &RawDelegationData::new(initial_delegation2.into(), env.block.height),
            )
            .unwrap();
        storage::mix_delegations(&mut deps.storage, &identity)
            .save(
                b"delegator3",
                &RawDelegationData::new(initial_delegation3.into(), env.block.height),
            )
            .unwrap();
        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;
        let bond_reward = current_state.mixnode_epoch_bond_reward;
        let delegation_reward = current_state.mixnode_epoch_delegation_reward;
        // the node's bond and delegations are correctly increased and scaled by uptime
        // if node was 100% up, it will get full epoch reward
        let expected_mix_reward = Uint128(initial_mix_bond) * bond_reward;
        let expected_delegation1_reward = Uint128(initial_delegation1) * delegation_reward;
        let expected_delegation2_reward = Uint128(initial_delegation2) * delegation_reward;
        let expected_delegation3_reward = Uint128(initial_delegation3) * delegation_reward;
        let expected_bond = expected_mix_reward + Uint128(initial_mix_bond);
        let expected_delegation1 = expected_delegation1_reward + Uint128(initial_delegation1);
        let expected_delegation2 = expected_delegation2_reward + Uint128(initial_delegation2);
        let expected_delegation3 = expected_delegation3_reward + Uint128(initial_delegation3);
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            identity.clone(),
            100,
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();
        assert_eq!(
            expected_bond,
            storage::read_mixnode_bond(deps.as_ref().storage, identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation1,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator1".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            expected_delegation2,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator2".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            expected_delegation3,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator3".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            vec![
                attr("bond increase", expected_mix_reward),
                attr(
                    "total delegation increase",
                    expected_delegation1_reward
                        + expected_delegation2_reward
                        + expected_delegation3_reward
                ),
            ],
            res.attributes
        );
        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_bond_reward = rewards_helpers::scale_reward_by_uptime(bond_reward, 20).unwrap();
        let scaled_delegation_reward =
            rewards_helpers::scale_reward_by_uptime(delegation_reward, 20).unwrap();
        let expected_mix_reward = expected_bond * scaled_bond_reward;
        let expected_delegation1_reward = expected_delegation1 * scaled_delegation_reward;
        let expected_delegation2_reward = expected_delegation2 * scaled_delegation_reward;
        let expected_delegation3_reward = expected_delegation3 * scaled_delegation_reward;
        let expected_bond = expected_mix_reward + expected_bond;
        let expected_delegation1 = expected_delegation1_reward + expected_delegation1;
        let expected_delegation2 = expected_delegation2_reward + expected_delegation2;
        let expected_delegation3 = expected_delegation3_reward + expected_delegation3;
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            identity.clone(),
            20,
            2,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 2).unwrap();
        assert_eq!(
            expected_bond,
            storage::read_mixnode_bond(deps.as_ref().storage, identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation1,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator1".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            expected_delegation2,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator2".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            expected_delegation3,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator3".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            vec![
                attr("bond increase", expected_mix_reward),
                attr(
                    "total delegation increase",
                    expected_delegation1_reward
                        + expected_delegation2_reward
                        + expected_delegation3_reward
                ),
            ],
            res.attributes
        );
        // if the node was 0% up, nobody will get any rewards
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 3).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            identity.clone(),
            0,
            3,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 3).unwrap();
        assert_eq!(
            expected_bond,
            storage::read_mixnode_bond(deps.as_ref().storage, identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation1,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator1".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            expected_delegation2,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator2".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            expected_delegation3,
            storage::mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator3".as_bytes())
                .unwrap()
                .amount
        );
        assert_eq!(
            vec![
                attr("bond increase", Uint128(0)),
                attr("total delegation increase", Uint128(0)),
            ],
            res.attributes
        );
    }
    #[cfg(test)]
    mod multi_delegations {
        use super::*;
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
    #[cfg(test)]
    mod finding_old_delegations {
        use super::*;
        use crate::mixnodes::delegation_transactions::total_delegations;
        use crate::support::tests::test_helpers::raw_delegation_fixture;
        use cosmwasm_std::Addr;
        #[test]
        fn when_there_werent_any() {
            let deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "nodeidentity".into();
            let read_bucket = storage::mix_delegations_read(&deps.storage, &node_identity);
            let old_delegations = total_delegations(read_bucket).unwrap();
            assert_eq!(Coin::new(0, DENOM), old_delegations);
        }
        #[test]
        fn when_some_existed() {
            let num_delegations = vec![
                1,
                5,
                OLD_DELEGATIONS_CHUNK_SIZE - 1,
                OLD_DELEGATIONS_CHUNK_SIZE,
                OLD_DELEGATIONS_CHUNK_SIZE + 1,
                OLD_DELEGATIONS_CHUNK_SIZE * 3,
                OLD_DELEGATIONS_CHUNK_SIZE * 3 + 1,
            ];
            for delegations in num_delegations {
                let mut deps = test_helpers::init_contract();
                let node_identity: IdentityKey = "nodeidentity".into();
                // delegate some stake
                let mut write_bucket = storage::mix_delegations(&mut deps.storage, &node_identity);
                for i in 1..=delegations {
                    let delegator = Addr::unchecked(format!("delegator{}", i));
                    let delegation = raw_delegation_fixture(i as u128);
                    write_bucket
                        .save(delegator.as_bytes(), &delegation)
                        .unwrap();
                }
                let read_bucket = storage::mix_delegations_read(&deps.storage, &node_identity);
                let old_delegations = total_delegations(read_bucket).unwrap();
                let total_delegation = (1..=delegations as u128).into_iter().sum();
                assert_eq!(Coin::new(total_delegation, DENOM), old_delegations);
            }
        }
    }
}
