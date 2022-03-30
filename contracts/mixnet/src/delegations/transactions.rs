// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use super::storage::{self, PENDING_DELEGATION_EVENTS};
use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use config::defaults::DENOM;
use cosmwasm_std::{
    coins, wasm_execute, Addr, Api, BankMsg, Coin, DepsMut, Env, Event, MessageInfo, Order,
    Response, Storage, Uint128, WasmMsg,
};
use mixnet_contract_common::events::{
    new_error_event, new_pending_delegation_event, new_pending_undelegation_event,
    new_undelegation_event,
};
use mixnet_contract_common::mixnode::{DelegationEvent, PendingUndelegate};
use mixnet_contract_common::{Delegation, IdentityKey};
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;
use vesting_contract_common::one_ucoin;

pub fn try_reconcile_all_delegation_events(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    _try_reconcile_all_delegation_events(deps.storage, deps.api)
}

// TODO: Error handling?
pub(crate) fn _try_reconcile_all_delegation_events(
    storage: &mut dyn Storage,
    api: &dyn Api,
) -> Result<Response, ContractError> {
    let pending_delegation_events = PENDING_DELEGATION_EVENTS
        .range(storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .collect::<Vec<((Vec<u8>, u64, String), DelegationEvent)>>();

    let mut response = Response::new();

    for (key, delegation_event) in pending_delegation_events {
        match delegation_event {
            DelegationEvent::Delegate(delegation) => {
                let event = try_reconcile_delegation(storage, delegation)?;
                response = response.add_event(event);
            }
            DelegationEvent::Undelegate(pending_undelegate) => {
                let undelegate_response =
                    try_reconcile_undelegation(storage, api, &pending_undelegate)?;
                response = response.add_event(undelegate_response.event);
                if let Some(msg) = undelegate_response.bank_msg {
                    response = response.add_message(msg);
                }
                if let Some(msg) = undelegate_response.wasm_msg {
                    response = response.add_message(msg);
                }
            }
        }
        PENDING_DELEGATION_EVENTS.remove(storage, key);
    }
    Ok(response)
}

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
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    let amount = validate_delegation_stake(info.funds)?;

    _try_delegate_to_mixnode(
        deps.storage,
        deps.api,
        env.block.height,
        &mix_identity,
        info.sender.as_str(),
        amount,
        None,
    )
}

pub(crate) fn try_delegate_to_mixnode_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    delegate: String,
) -> Result<Response, ContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    let amount = validate_delegation_stake(info.funds)?;

    _try_delegate_to_mixnode(
        deps.storage,
        deps.api,
        env.block.height,
        &mix_identity,
        &delegate,
        amount,
        Some(info.sender),
    )
}

pub(crate) fn try_reconcile_delegation(
    storage: &mut dyn Storage,
    delegation: Delegation,
) -> Result<Event, ContractError> {
    // update total_delegation of this node
    mixnodes_storage::TOTAL_DELEGATION.update::<_, ContractError>(
        storage,
        &delegation.node_identity,
        |total_delegation| {
            // since we know that the target node exists and because the total_delegation bucket
            // entry is created whenever the node itself is added, the unwrap here is fine
            // as the entry MUST exist
            Ok(total_delegation.unwrap() + delegation.amount.amount)
        },
    )?;

    // update [or create new] pending delegation of this delegator
    storage::delegations().update::<_, ContractError>(
        storage,
        delegation.storage_key(),
        |existing_delegation| {
            Ok(match existing_delegation {
                Some(mut existing_delegation) => {
                    existing_delegation
                        .increment_amount(delegation.amount.amount, Some(delegation.block_height));
                    existing_delegation
                }
                None => delegation.clone(),
            })
        },
    )?;

    Ok(new_pending_delegation_event(
        &delegation.owner,
        &delegation.proxy,
        &delegation.amount,
        &delegation.node_identity,
    ))
}

pub(crate) fn _try_delegate_to_mixnode(
    storage: &mut dyn Storage,
    api: &dyn Api,
    block_height: u64,
    mix_identity: &str,
    delegate: &str,
    amount: Coin,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let delegate = api.addr_validate(delegate)?;

    // check if the target node actually exists
    if mixnodes_storage::mixnodes()
        .may_load(storage, mix_identity)?
        .is_none()
    {
        return Err(ContractError::MixNodeBondNotFound {
            identity: mix_identity.to_string(),
        });
    }

    let delegation = Delegation::new(
        delegate.to_owned(),
        mix_identity.to_string(),
        amount.clone(),
        block_height,
        proxy.clone(),
    );

    storage::PENDING_DELEGATION_EVENTS.save(
        storage,
        delegation.event_storage_key(),
        &DelegationEvent::Delegate(delegation),
    )?;

    Ok(Response::new().add_event(new_pending_delegation_event(
        &delegate,
        &proxy,
        &amount,
        mix_identity,
    )))
}

pub(crate) fn try_remove_delegation_from_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    _try_remove_delegation_from_mixnode(deps, env, mix_identity, info.sender.as_str(), None)
}

pub(crate) fn try_remove_delegation_from_mixnode_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    delegate: String,
) -> Result<Response, ContractError> {
    _try_remove_delegation_from_mixnode(deps, env, mix_identity, &delegate, Some(info.sender))
}

pub struct ReconcileUndelegateResponse {
    bank_msg: Option<BankMsg>,
    wasm_msg: Option<WasmMsg>,
    event: Event,
}

pub(crate) fn try_reconcile_undelegation(
    storage: &mut dyn Storage,
    api: &dyn Api,
    pending_undelegate: &PendingUndelegate,
) -> Result<ReconcileUndelegateResponse, ContractError> {
    let delegation_map = storage::delegations();

    let any_delegations = delegation_map
        .prefix(pending_undelegate.storage_key())
        .keys(storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|v| v.ok())
        .next()
        .is_some();

    if !any_delegations {
        return Ok(ReconcileUndelegateResponse {
            bank_msg: None,
            wasm_msg: None,
            event: new_error_event(
                ContractError::NoMixnodeDelegationFound {
                    identity: pending_undelegate.mix_identity(),
                    address: pending_undelegate.delegate().to_string(),
                }
                .to_string(),
            ),
        });
    }

    let reward = crate::rewards::transactions::_try_compound_delegator_reward(
        pending_undelegate.block_height(),
        api,
        storage,
        pending_undelegate.delegate().as_str(),
        &pending_undelegate.mix_identity(),
        None,
    )?;

    // Might want to introduce paging here
    let delegation_heights = delegation_map
        .prefix(pending_undelegate.storage_key())
        .keys(storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|v| v.ok())
        .collect::<Vec<u64>>();

    if delegation_heights.is_empty() {
        return Ok(ReconcileUndelegateResponse {
            bank_msg: None,
            wasm_msg: None,
            event: new_error_event(
                ContractError::NoMixnodeDelegationFound {
                    identity: pending_undelegate.mix_identity(),
                    address: pending_undelegate.delegate().to_string(),
                }
                .to_string(),
            ),
        });
    }

    let mut total_delegation = Uint128::zero();

    if crate::mixnodes::storage::mixnodes()
        .may_load(storage, &pending_undelegate.mix_identity())?
        .is_none()
    {
        // Since the mixnode is no longer bonded the reward did not compound and we need to manually add it to the total
        total_delegation = reward;
    }

    for h in delegation_heights {
        let delegation =
            delegation_map.load(storage, pending_undelegate.delegation_key(h).clone())?;
        total_delegation += delegation.amount.amount;
        delegation_map.replace(
            storage,
            pending_undelegate.delegation_key(h),
            None,
            Some(&delegation),
        )?;
    }

    let bank_msg = BankMsg::Send {
        to_address: pending_undelegate
            .proxy()
            .as_ref()
            .unwrap_or(&pending_undelegate.delegate())
            .to_string(),
        amount: coins(total_delegation.u128(), DENOM),
    };

    mixnodes_storage::TOTAL_DELEGATION.update::<_, ContractError>(
        storage,
        &pending_undelegate.mix_identity(),
        |total_node_delegation| {
            // the first unwrap is fine because the delegation information MUST exist, otherwise we would
            // have never gotten here in the first place
            // the second unwrap is also fine because we should NEVER underflow here,
            // if we do, it means we have some serious error in our logic
            Ok(total_node_delegation
                .unwrap()
                .checked_sub(total_delegation)
                .unwrap())
        },
    )?;

    let mut wasm_msg = None;

    if let Some(proxy) = &pending_undelegate.proxy() {
        let msg = Some(VestingContractExecuteMsg::TrackUndelegation {
            owner: pending_undelegate.delegate().as_str().to_string(),
            mix_identity: pending_undelegate.mix_identity(),
            amount: Coin::new(total_delegation.u128(), DENOM),
        });

        wasm_msg = Some(wasm_execute(proxy, &msg, vec![one_ucoin()])?);
    }

    let event = new_undelegation_event(
        &pending_undelegate.delegate(),
        &pending_undelegate.proxy(),
        &pending_undelegate.mix_identity(),
        total_delegation,
    );

    Ok(ReconcileUndelegateResponse {
        bank_msg: Some(bank_msg),
        wasm_msg,
        event,
    })
}

pub(crate) fn _try_remove_delegation_from_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    mix_identity: IdentityKey,
    delegate: &str,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let delegate = deps.api.addr_validate(delegate)?;

    let event = PendingUndelegate::new(
        mix_identity.to_string(),
        delegate.clone(),
        proxy.clone(),
        env.block.height,
    );

    PENDING_DELEGATION_EVENTS.save(
        deps.storage,
        event.event_storage_key(),
        &DelegationEvent::Undelegate(event),
    )?;

    Ok(Response::new().add_event(new_pending_undelegation_event(
        &delegate,
        &proxy,
        &mix_identity,
    )))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coins;

    use crate::support::tests;
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
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            let delegation = coin(123, DENOM);
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone(),
            )
            .is_ok());

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                delegation.clone(),
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(
                    &deps.storage,
                    &identity,
                    delegation_owner.as_bytes(),
                    mock_env().block.height
                )
                .unwrap()
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
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            try_remove_mixnode(mock_env(), deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
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
            test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            try_remove_mixnode(mock_env(), deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation = coin(123, DENOM);
            let delegation_owner = Addr::unchecked("sender");
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone(),
            )
            .is_ok());

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                delegation.clone(),
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(
                    &deps.storage,
                    &identity,
                    delegation_owner.as_bytes(),
                    mock_env().block.height
                )
                .unwrap()
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
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            let delegation1 = coin(100, DENOM);
            let delegation2 = coin(50, DENOM);

            let mut env = mock_env();

            try_delegate_to_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(delegation_owner.as_str(), &[delegation1.clone()]),
                identity.clone(),
            )
            .unwrap();

            env.block.height += 1;

            try_delegate_to_mixnode(
                deps.as_mut(),
                env,
                mock_info(delegation_owner.as_str(), &[delegation2.clone()]),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            // let expected = Delegation::new(
            //     delegation_owner.clone(),
            //     identity.clone(),
            //     coin(delegation1.amount.u128() + delegation2.amount.u128(), DENOM),
            //     mock_env().block.height,
            //     None,
            // );

            // assert_eq!(
            //     expected,
            //     test_helpers::read_delegation(
            //         &deps.storage,
            //         &identity,
            //         delegation_owner.as_bytes(),
            //         mock_env().block.height
            //     )
            //     .unwrap()
            // );

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
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
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
                env1.clone(),
                mock_info(delegation_owner.as_str(), &[delegation.clone()]),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            assert_eq!(
                initial_height,
                test_helpers::read_delegation(
                    &deps.storage,
                    &identity,
                    delegation_owner.as_bytes(),
                    env1.block.height
                )
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

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            let delegations = crate::delegations::queries::query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                identity,
                delegation_owner.to_string(),
                None,
            )
            .unwrap();

            let total_delegation = delegations
                .iter()
                .fold(Uint128::zero(), |acc, d| acc + d.amount.amount);

            assert_eq!(delegation.amount + delegation.amount, total_delegation);
        }

        #[test]
        fn block_height_is_not_updated_on_different_delegator() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
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
                env1.clone(),
                mock_info(delegation_owner1.as_str(), &[delegation1]),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            assert_eq!(
                initial_height,
                test_helpers::read_delegation(
                    &deps.storage,
                    &identity,
                    delegation_owner1.as_bytes(),
                    env1.block.height
                )
                .unwrap()
                .block_height
            );
            try_delegate_to_mixnode(
                deps.as_mut(),
                env2.clone(),
                mock_info(delegation_owner2.as_str(), &[delegation2]),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            assert_eq!(
                initial_height,
                test_helpers::read_delegation(
                    &deps.storage,
                    &identity,
                    delegation_owner1.as_bytes(),
                    env1.block.height
                )
                .unwrap()
                .block_height
            );
            assert_eq!(
                second_height,
                test_helpers::read_delegation(
                    &deps.storage,
                    identity,
                    delegation_owner2.as_bytes(),
                    env2.block.height
                )
                .unwrap()
                .block_height
            );
        }

        #[test]
        fn is_disallowed_for_already_delegated_node_if_it_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();
            try_remove_mixnode(mock_env(), deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
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
            let identity1 = test_helpers::add_mixnode(
                mixnode_owner1,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let identity2 = test_helpers::add_mixnode(
                mixnode_owner2,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
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

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

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
                test_helpers::read_delegation(
                    &deps.storage,
                    identity1,
                    delegation_owner.as_bytes(),
                    mock_env().block.height
                )
                .unwrap()
            );
            assert_eq!(
                expected2,
                test_helpers::read_delegation(
                    &deps.storage,
                    identity2,
                    delegation_owner.as_bytes(),
                    mock_env().block.height
                )
                .unwrap()
            );
        }

        #[test]
        fn is_allowed_by_multiple_users() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
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
            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

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
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            let delegation_amount = coin(100, DENOM);
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &[delegation_amount.clone()]),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            try_remove_mixnode(mock_env(), deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            let expected = Delegation::new(
                delegation_owner.clone(),
                identity.clone(),
                delegation_amount,
                mock_env().block.height,
                None,
            );

            assert_eq!(
                expected,
                test_helpers::read_delegation(
                    &deps.storage,
                    identity,
                    delegation_owner.as_bytes(),
                    mock_env().block.height
                )
                .unwrap()
            )
        }
    }

    #[cfg(test)]
    mod removing_mix_stake_delegation {
        use crate::delegations::queries::query_mixnode_delegation;
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::mock_env;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;
        use cosmwasm_std::Uint128;

        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests;

        use super::storage;
        use super::*;

        // TODO: Probably delete due to reconciliation logic
        #[ignore]
        #[test]
        fn fails_if_delegation_never_existed() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mixnode_owner = "bob";
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            assert_eq!(
                Err(ContractError::NoMixnodeDelegationFound {
                    identity: identity.clone(),
                    address: delegation_owner.to_string(),
                }),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    env,
                    mock_info(delegation_owner.as_str(), &[]),
                    identity,
                )
            );
        }

        // TODO: Update to work with reconciliation
        #[ignore]
        #[test]
        fn succeeds_if_delegation_existed() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let env = mock_env();
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            let _delegation = query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                identity.clone(),
                delegation_owner.clone().into_string(),
                None,
            )
            .unwrap();

            let expected_response = Response::new()
                .add_message(BankMsg::Send {
                    to_address: delegation_owner.clone().into(),
                    amount: coins(100, DENOM),
                })
                .add_event(new_undelegation_event(
                    &delegation_owner,
                    &None,
                    &identity,
                    Uint128::new(100),
                ));

            assert_eq!(
                Ok(expected_response),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    env,
                    mock_info(delegation_owner.as_str(), &[]),
                    identity.clone(),
                )
            );
            assert!(storage::delegations()
                .may_load(
                    &deps.storage,
                    (identity.clone(), delegation_owner.as_bytes().to_vec(), 0),
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

        // TODO: Update to work with reconciliation
        #[ignore]
        #[test]
        fn succeeds_if_delegation_existed_even_if_node_unbonded() {
            let mut deps = test_helpers::init_contract();
            let mixnode_owner = "bob";
            let env = mock_env();
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner = Addr::unchecked("sender");
            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            let delegation = query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                identity.clone(),
                delegation_owner.clone().into_string(),
                None,
            )
            .unwrap();

            let expected_response = Response::new()
                .add_message(BankMsg::Send {
                    to_address: delegation_owner.clone().into(),
                    amount: coins(100, DENOM),
                })
                .add_event(new_undelegation_event(
                    &delegation_owner,
                    &None,
                    &identity,
                    Uint128::new(100),
                ));

            try_remove_mixnode(mock_env(), deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            assert_eq!(
                Ok(expected_response),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    env,
                    mock_info(delegation_owner.as_str(), &[]),
                    identity.clone(),
                )
            );

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            assert!(test_helpers::read_delegation(
                &deps.storage,
                identity,
                delegation_owner.as_bytes(),
                mock_env().block.height
            )
            .is_none());
        }

        #[test]
        fn total_delegation_is_preserved_if_only_some_undelegate() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mixnode_owner = "bob";
            let identity = test_helpers::add_mixnode(
                mixnode_owner,
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );
            let delegation_owner1 = Addr::unchecked("sender1");
            let delegation_owner2 = Addr::unchecked("sender2");
            let delegation1 = coin(123, DENOM);
            let delegation2 = coin(234, DENOM);
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner1.as_str(), &[delegation1]),
                identity.clone(),
            )
            .is_ok());

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner2.as_str(), &[delegation2.clone()]),
                identity.clone(),
            )
            .is_ok());

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();

            // sender1 undelegates
            try_remove_delegation_from_mixnode(
                deps.as_mut(),
                env,
                mock_info(delegation_owner1.as_str(), &[]),
                identity.clone(),
            )
            .unwrap();

            _try_reconcile_all_delegation_events(&mut deps.storage, &deps.api).unwrap();
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
