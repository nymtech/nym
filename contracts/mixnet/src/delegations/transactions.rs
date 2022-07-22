// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use crate::support::helpers::validate_delegation_stake;
use cosmwasm_std::{Addr, Coin, DepsMut, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_pending_delegation_event, new_pending_undelegation_event,
};
use mixnet_contract_common::pending_events::PendingEpochEvent;
use mixnet_contract_common::{Delegation, NodeId};

pub(crate) fn try_delegate_to_mixnode(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    _try_delegate_to_mixnode(deps, mix_id, info.sender, info.funds, None)
}

pub(crate) fn try_delegate_to_mixnode_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
    delegate: String,
) -> Result<Response, MixnetContractError> {
    let delegate = deps.api.addr_validate(&delegate)?;
    _try_delegate_to_mixnode(deps, mix_id, delegate, info.funds, Some(info.sender))
}

pub(crate) fn _try_delegate_to_mixnode(
    deps: DepsMut<'_>,
    mix_id: NodeId,
    delegate: Addr,
    amount: Vec<Coin>,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    let contract_state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
    let delegation = validate_delegation_stake(
        amount,
        contract_state.params.minimum_mixnode_delegation,
        contract_state.rewarding_denom,
    )?;

    // check if the target node actually exists and is still bonded
    match mixnodes_storage::mixnode_bonds().may_load(deps.storage, mix_id)? {
        None => return Err(MixnetContractError::MixNodeBondNotFound { id: mix_id }),
        Some(bond) if bond.is_unbonding => {
            return Err(MixnetContractError::MixnodeIsUnbonding { node_id: mix_id })
        }
        _ => (),
    }

    // push the event onto the queue and wait for it to be picked up at the end of the epoch
    let cosmos_event = new_pending_delegation_event(&delegate, &proxy, &delegation, mix_id);

    let epoch_event = PendingEpochEvent::Delegate {
        owner: delegate,
        mix_id,
        amount: delegation,
        proxy,
    };
    interval_storage::push_new_epoch_event(deps.storage, &epoch_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn try_remove_delegation_from_mixnode(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    _try_remove_delegation_from_mixnode(deps, mix_id, info.sender, None)
}

pub(crate) fn try_remove_delegation_from_mixnode_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
    delegate: String,
) -> Result<Response, MixnetContractError> {
    let delegate = deps.api.addr_validate(&delegate)?;
    _try_remove_delegation_from_mixnode(deps, mix_id, delegate, Some(info.sender))
}

pub(crate) fn _try_remove_delegation_from_mixnode(
    deps: DepsMut<'_>,
    mix_id: NodeId,
    delegate: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // see if the delegation even exists
    let storage_key = Delegation::generate_storage_key(mix_id, &delegate, proxy.as_ref());

    if storage::delegations()
        .may_load(deps.storage, storage_key)?
        .is_none()
    {
        return Err(MixnetContractError::NoMixnodeDelegationFound {
            mix_id,
            address: delegate.into_string(),
            proxy: proxy.map(Addr::into_string),
        });
    }

    // push the event onto the queue and wait for it to be picked up at the end of the epoch
    let cosmos_event = new_pending_undelegation_event(&delegate, &proxy, mix_id);

    let epoch_event = PendingEpochEvent::Undelegate {
        owner: delegate,
        mix_id,
        proxy,
    };
    interval_storage::push_new_epoch_event(deps.storage, &epoch_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod delegating_to_mixnode {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::rewards::storage as rewards_storage;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{coin, Decimal};

        #[test]
        fn can_only_be_done_towards_an_existing_mixnode() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let sender = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);

            let res = try_delegate_to_mixnode(test.deps_mut(), sender, 42);
            assert_eq!(
                res,
                Err(MixnetContractError::MixNodeBondNotFound { id: 42 })
            )
        }

        #[test]
        fn must_contain_non_zero_amount_of_coins() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(0, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[]);
            let sender3 = mock_info(owner, &[coin(1000, "some-weird-coin")]);

            let res = try_delegate_to_mixnode(test.deps_mut(), sender1, mix_id);
            assert_eq!(res, Err(MixnetContractError::EmptyDelegation));
            let res = try_delegate_to_mixnode(test.deps_mut(), sender2, mix_id);
            assert_eq!(res, Err(MixnetContractError::EmptyDelegation));
            let res = try_delegate_to_mixnode(test.deps_mut(), sender3, mix_id);
            assert_eq!(
                res,
                Err(MixnetContractError::WrongDenom {
                    received: "some-weird-coin".to_string(),
                    expected: TEST_COIN_DENOM.to_string()
                })
            );
        }

        #[test]
        fn if_applicable_must_contain_at_least_the_minimum_pledge() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[coin(150_000_000, TEST_COIN_DENOM)]);

            let min_delegation = coin(150_000_000, TEST_COIN_DENOM);
            let mut contract_state = mixnet_params_storage::CONTRACT_STATE
                .load(test.deps().storage)
                .unwrap();
            contract_state.params.minimum_mixnode_delegation = Some(min_delegation.clone());
            mixnet_params_storage::CONTRACT_STATE
                .save(test.deps_mut().storage, &contract_state)
                .unwrap();

            let res = try_delegate_to_mixnode(test.deps_mut(), sender1, mix_id);
            assert_eq!(
                res,
                Err(MixnetContractError::InsufficientDelegation {
                    received: coin(100_000_000, TEST_COIN_DENOM),
                    minimum: min_delegation
                })
            );

            let res = try_delegate_to_mixnode(test.deps_mut(), sender2, mix_id);
            assert!(res.is_ok())
        }

        #[test]
        fn can_only_be_done_towards_fully_bonded_mixnode() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let sender = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);

            let mix_id_unbonding = test.add_dummy_mixnode("mix-owner-unbonding", None);
            let mix_id_unbonded = test.add_dummy_mixnode("mix-owner-unbonded", None);
            let mix_id_unbonded_leftover =
                test.add_dummy_mixnode("mix-owner-unbonded-leftover", None);

            // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
            let mut rewarding_details = rewards_storage::MIXNODE_REWARDING
                .load(test.deps().storage, mix_id_unbonded_leftover)
                .unwrap();
            rewarding_details.delegates = Decimal::raw(12345);
            rewarding_details.unique_delegations = 1;
            rewards_storage::MIXNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    mix_id_unbonded_leftover,
                    &rewarding_details,
                )
                .unwrap();

            try_remove_mixnode(test.deps_mut(), mock_info("mix-owner-unbonded", &[])).unwrap();
            try_remove_mixnode(
                test.deps_mut(),
                mock_info("mix-owner-unbonded-leftover", &[]),
            )
            .unwrap();

            test.execute_all_pending_events();
            try_remove_mixnode(test.deps_mut(), mock_info("mix-owner-unbonding", &[])).unwrap();

            let res = try_delegate_to_mixnode(test.deps_mut(), sender.clone(), mix_id_unbonding);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    node_id: mix_id_unbonding
                })
            );

            let res = try_delegate_to_mixnode(test.deps_mut(), sender.clone(), mix_id_unbonded);
            assert_eq!(
                res,
                Err(MixnetContractError::MixNodeBondNotFound {
                    id: mix_id_unbonded
                })
            );

            let res = try_delegate_to_mixnode(test.deps_mut(), sender, mix_id_unbonded_leftover);
            assert_eq!(
                res,
                Err(MixnetContractError::MixNodeBondNotFound {
                    id: mix_id_unbonded_leftover
                })
            );
        }

        #[test]
        fn can_still_be_done_if_prior_delegation_exists() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[coin(50_000_000, TEST_COIN_DENOM)]);

            let res = try_delegate_to_mixnode(test.deps_mut(), sender1, mix_id);
            assert!(res.is_ok());

            // still OK
            let res = try_delegate_to_mixnode(test.deps_mut(), sender2, mix_id);
            assert!(res.is_ok())
        }

        #[test]
        fn correctly_pushes_appropriate_epoch_event() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let amount1 = coin(100_000_000, TEST_COIN_DENOM);
            let amount2 = coin(50_000_000, TEST_COIN_DENOM);

            let sender1 = mock_info(owner, &[amount1.clone()]);
            let sender2 = mock_info(test.vesting_contract().as_str(), &[amount2.clone()]);

            try_delegate_to_mixnode(test.deps_mut(), sender1, mix_id).unwrap();
            try_delegate_to_mixnode_on_behalf(test.deps_mut(), sender2, mix_id, owner.into())
                .unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0],
                PendingEpochEvent::Delegate {
                    owner: Addr::unchecked(owner),
                    mix_id,
                    amount: amount1,
                    proxy: None
                }
            );

            assert_eq!(
                events[1],
                PendingEpochEvent::Delegate {
                    owner: Addr::unchecked(owner),
                    mix_id,
                    amount: amount2,
                    proxy: Some(test.vesting_contract())
                }
            );
        }
    }

    #[cfg(test)]
    mod removing_mixnode_delegation {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::mock_info;

        #[test]
        fn cannot_be_performed_if_delegation_never_existed() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let sender = mock_info(owner, &[]);
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let res = try_remove_delegation_from_mixnode(test.deps_mut(), sender, mix_id);
            assert_eq!(
                res,
                Err(MixnetContractError::NoMixnodeDelegationFound {
                    mix_id,
                    address: owner.to_string(),
                    proxy: None
                })
            )
        }

        #[test]
        fn cannot_be_performed_if_the_delegation_is_still_pending() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[]);

            try_delegate_to_mixnode(test.deps_mut(), sender1, mix_id).unwrap();

            let res = try_remove_delegation_from_mixnode(test.deps_mut(), sender2, mix_id);
            assert_eq!(
                res,
                Err(MixnetContractError::NoMixnodeDelegationFound {
                    mix_id,
                    address: owner.to_string(),
                    proxy: None
                })
            )
        }

        #[test]
        fn as_long_as_delegation_exists_can_always_be_performed() {
            let mut test = TestSetup::new();
            let owner = "delegator";
            let sender = mock_info(owner, &[]);

            let normal_mix_id = test.add_dummy_mixnode("mix-owner", None);
            let mix_id_unbonding = test.add_dummy_mixnode("mix-owner-unbonding", None);
            let mix_id_unbonded_leftover =
                test.add_dummy_mixnode("mix-owner-unbonded-leftover", None);

            test.add_immediate_delegation(owner, 10000u32, normal_mix_id);
            test.add_immediate_delegation(owner, 10000u32, mix_id_unbonding);
            test.add_immediate_delegation(owner, 10000u32, mix_id_unbonded_leftover);

            try_remove_mixnode(
                test.deps_mut(),
                mock_info("mix-owner-unbonded-leftover", &[]),
            )
            .unwrap();

            test.execute_all_pending_events();
            try_remove_mixnode(test.deps_mut(), mock_info("mix-owner-unbonding", &[])).unwrap();

            let res =
                try_remove_delegation_from_mixnode(test.deps_mut(), sender.clone(), normal_mix_id);
            assert!(res.is_ok());

            let res = try_remove_delegation_from_mixnode(
                test.deps_mut(),
                sender.clone(),
                mix_id_unbonding,
            );
            assert!(res.is_ok());

            let res = try_remove_delegation_from_mixnode(
                test.deps_mut(),
                sender,
                mix_id_unbonded_leftover,
            );
            assert!(res.is_ok());
        }
    }
}
