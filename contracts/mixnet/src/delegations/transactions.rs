// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use crate::support::helpers::{ensure_epoch_in_progress_state, validate_delegation_stake};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_pending_delegation_event, new_pending_undelegation_event,
};
use mixnet_contract_common::pending_events::PendingEpochEventKind;
use mixnet_contract_common::{Delegation, MixId};

pub(crate) fn try_delegate_to_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_id: MixId,
) -> Result<Response, MixnetContractError> {
    // delegation is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    // check if the delegation contains any funds of the appropriate denomination
    let contract_state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
    let delegation = validate_delegation_stake(
        info.funds,
        contract_state.params.minimum_mixnode_delegation,
        contract_state.rewarding_denom,
    )?;

    // check if the target node actually exists and is still bonded
    match mixnodes_storage::mixnode_bonds().may_load(deps.storage, mix_id)? {
        None => return Err(MixnetContractError::MixNodeBondNotFound { mix_id }),
        Some(bond) if bond.is_unbonding => {
            return Err(MixnetContractError::MixnodeIsUnbonding { mix_id })
        }
        _ => (),
    }

    // push the event onto the queue and wait for it to be picked up at the end of the epoch
    let cosmos_event = new_pending_delegation_event(&info.sender, &delegation, mix_id);

    let epoch_event = PendingEpochEventKind::new_delegate(info.sender, mix_id, delegation);
    interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn try_remove_delegation_from_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_id: MixId,
) -> Result<Response, MixnetContractError> {
    // undelegation is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    // see if the delegation even exists
    let storage_key = Delegation::generate_storage_key(mix_id, &info.sender, None);

    if storage::delegations()
        .may_load(deps.storage, storage_key)?
        .is_none()
    {
        return Err(MixnetContractError::NoMixnodeDelegationFound {
            mix_id,
            address: info.sender.into_string(),
            proxy: None,
        });
    }

    // push the event onto the queue and wait for it to be picked up at the end of the epoch
    let cosmos_event = new_pending_undelegation_event(&info.sender, mix_id);

    let epoch_event = PendingEpochEventKind::new_undelegate(info.sender, mix_id);
    interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;

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
        use cosmwasm_std::{coin, Addr, Decimal};
        use mixnet_contract_common::{EpochState, EpochStatus};

        #[test]
        fn cant_be_performed_if_epoch_transition_is_in_progress() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let env = test.env();

                let owner = "delegator";
                let mix_id = test.add_dummy_mixnode("mix-owner", None);
                let sender = mock_info(owner, &[coin(50_000_000, TEST_COIN_DENOM)]);

                let res = try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender, mix_id);
                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn can_only_be_done_towards_an_existing_mixnode() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "delegator";
            let sender = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);

            let res = try_delegate_to_mixnode(test.deps_mut(), env, sender, 42);
            assert_eq!(
                res,
                Err(MixnetContractError::MixNodeBondNotFound { mix_id: 42 })
            )
        }

        #[test]
        fn must_contain_non_zero_amount_of_coins() {
            let mut test = TestSetup::new();
            let env = test.env();

            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(0, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[]);
            let sender3 = mock_info(owner, &[coin(1000, "some-weird-coin")]);

            let res = try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender1, mix_id);
            assert_eq!(res, Err(MixnetContractError::EmptyDelegation));
            let res = try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender2, mix_id);
            assert_eq!(res, Err(MixnetContractError::EmptyDelegation));
            let res = try_delegate_to_mixnode(test.deps_mut(), env, sender3, mix_id);
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
            let env = test.env();

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

            let res = try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender1, mix_id);
            assert_eq!(
                res,
                Err(MixnetContractError::InsufficientDelegation {
                    received: coin(100_000_000, TEST_COIN_DENOM),
                    minimum: min_delegation
                })
            );

            let res = try_delegate_to_mixnode(test.deps_mut(), env, sender2, mix_id);
            assert!(res.is_ok())
        }

        #[test]
        fn can_only_be_done_towards_fully_bonded_mixnode() {
            let mut test = TestSetup::new();
            let env = test.env();
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

            try_remove_mixnode(
                test.deps_mut(),
                env.clone(),
                mock_info("mix-owner-unbonded", &[]),
            )
            .unwrap();
            try_remove_mixnode(
                test.deps_mut(),
                env.clone(),
                mock_info("mix-owner-unbonded-leftover", &[]),
            )
            .unwrap();

            test.execute_all_pending_events();
            try_remove_mixnode(
                test.deps_mut(),
                env.clone(),
                mock_info("mix-owner-unbonding", &[]),
            )
            .unwrap();

            let res = try_delegate_to_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                mix_id_unbonding,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: mix_id_unbonding
                })
            );

            let res = try_delegate_to_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                mix_id_unbonded,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::MixNodeBondNotFound {
                    mix_id: mix_id_unbonded
                })
            );

            let res =
                try_delegate_to_mixnode(test.deps_mut(), env, sender, mix_id_unbonded_leftover);
            assert_eq!(
                res,
                Err(MixnetContractError::MixNodeBondNotFound {
                    mix_id: mix_id_unbonded_leftover
                })
            );
        }

        #[test]
        fn can_still_be_done_if_prior_delegation_exists() {
            let mut test = TestSetup::new();
            let env = test.env();

            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[coin(50_000_000, TEST_COIN_DENOM)]);

            let res = try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender1, mix_id);
            assert!(res.is_ok());

            // still OK
            let res = try_delegate_to_mixnode(test.deps_mut(), env, sender2, mix_id);
            assert!(res.is_ok())
        }

        #[test]
        fn correctly_pushes_appropriate_epoch_event() {
            let mut test = TestSetup::new();
            let env = test.env();

            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let amount1 = coin(100_000_000, TEST_COIN_DENOM);

            let sender1 = mock_info(owner, &[amount1.clone()]);

            try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender1, mix_id).unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0].kind,
                PendingEpochEventKind::new_delegate(Addr::unchecked(owner), mix_id, amount1,)
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
        use mixnet_contract_common::{EpochState, EpochStatus};

        #[test]
        fn cant_be_performed_if_epoch_transition_is_in_progress() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let mix_id = test.add_dummy_mixnode("owner", None);
                test.add_immediate_delegation("foomp", 1000u32, mix_id);

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let env = test.env();
                let res = try_remove_delegation_from_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    mock_info("sender", &[]),
                    mix_id,
                );
                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn cannot_be_performed_if_delegation_never_existed() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "delegator";
            let sender = mock_info(owner, &[]);
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let res = try_remove_delegation_from_mixnode(test.deps_mut(), env, sender, mix_id);
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
            let env = test.env();

            let owner = "delegator";
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender1 = mock_info(owner, &[coin(100_000_000, TEST_COIN_DENOM)]);
            let sender2 = mock_info(owner, &[]);

            try_delegate_to_mixnode(test.deps_mut(), env.clone(), sender1, mix_id).unwrap();

            let res = try_remove_delegation_from_mixnode(test.deps_mut(), env, sender2, mix_id);
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
            let env = test.env();

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
                env.clone(),
                mock_info("mix-owner-unbonded-leftover", &[]),
            )
            .unwrap();

            test.execute_all_pending_events();
            try_remove_mixnode(
                test.deps_mut(),
                env.clone(),
                mock_info("mix-owner-unbonding", &[]),
            )
            .unwrap();

            let res = try_remove_delegation_from_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                normal_mix_id,
            );
            assert!(res.is_ok());

            let res = try_remove_delegation_from_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                mix_id_unbonding,
            );
            assert!(res.is_ok());

            let res = try_remove_delegation_from_mixnode(
                test.deps_mut(),
                env,
                sender,
                mix_id_unbonded_leftover,
            );
            assert!(res.is_ok());
        }
    }
}
