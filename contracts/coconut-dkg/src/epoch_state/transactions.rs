// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{current_dealers, past_dealers};
use crate::dealings::storage::DEALINGS_BYTES;
use crate::epoch_state::storage::{CURRENT_EPOCH, THRESHOLD};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::STATE;
use coconut_dkg_common::types::{Epoch, EpochState};
use cosmwasm_std::{DepsMut, Env, Order, Response, Storage};

fn reset_epoch_state(storage: &mut dyn Storage) -> Result<(), ContractError> {
    THRESHOLD.remove(storage);
    let dealers: Vec<_> = current_dealers()
        .keys(storage, None, None, Order::Ascending)
        .collect::<Result<_, _>>()?;

    for dealer_addr in dealers {
        let details = current_dealers().load(storage, &dealer_addr)?;
        for dealings in DEALINGS_BYTES {
            dealings.remove(storage, &details.address);
        }
        current_dealers().remove(storage, &dealer_addr)?;
        past_dealers().save(storage, &dealer_addr, &details)?;
    }
    Ok(())
}

fn dealers_still_active(deps: &DepsMut<'_>) -> Result<usize, ContractError> {
    let state = STATE.load(deps.storage)?;
    let mut still_active = 0;
    for dealer_addr in current_dealers()
        .keys(deps.storage, None, None, Order::Ascending)
        .flatten()
    {
        if state
            .group_addr
            .is_voting_member(&deps.querier, &dealer_addr, None)?
            .is_some()
        {
            still_active += 1;
        }
    }
    Ok(still_active)
}

fn dealers_eq_members(deps: &DepsMut<'_>) -> Result<bool, ContractError> {
    let dealers_still_active = dealers_still_active(&deps)?;
    let all_dealers = current_dealers()
        .keys(deps.storage, None, None, Order::Ascending)
        .count();
    let group_members = STATE
        .load(deps.storage)?
        .group_addr
        .list_members(&deps.querier, None, None)?
        .len();

    Ok(dealers_still_active == all_dealers && all_dealers == group_members)
}

pub(crate) fn advance_epoch_state(deps: DepsMut<'_>, env: Env) -> Result<Response, ContractError> {
    let epoch = CURRENT_EPOCH.load(deps.storage)?;
    if epoch.finish_timestamp > env.block.time {
        return Err(ContractError::EarlyEpochStateAdvancement(
            epoch
                .finish_timestamp
                .minus_seconds(env.block.time.seconds())
                .seconds(),
        ));
    }

    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;
    let next_epoch = if let Some(state) = current_epoch.state.next() {
        // We are during DKG process
        if state == (EpochState::DealingExchange { resharing: false }) {
            let current_dealer_count = current_dealers()
                .keys(deps.storage, None, None, Order::Ascending)
                .count();
            // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
            let threshold = (2 * current_dealer_count as u64 + 3 - 1) / 3;
            THRESHOLD.save(deps.storage, &threshold)?;
        }
        Epoch::new(
            state,
            current_epoch.epoch_id,
            current_epoch.time_configuration,
            env.block.time,
        )
    } else if dealers_eq_members(&deps)? {
        // The dealer set hasn't changed, so we only extend the finish timestamp
        // The epoch remains the same, as we use it as key for storing VKs
        Epoch::new(
            current_epoch.state,
            current_epoch.epoch_id,
            current_epoch.time_configuration,
            env.block.time,
        )
    } else {
        // Dealer set changed, we need to redo DKG from scratch
        reset_epoch_state(deps.storage)?;
        Epoch::new(
            EpochState::default(),
            current_epoch.epoch_id + 1,
            current_epoch.time_configuration,
            env.block.time,
        )
    };
    CURRENT_EPOCH.save(deps.storage, &next_epoch)?;

    Ok(Response::default())
}

pub(crate) fn try_surpassed_threshold(
    deps: DepsMut<'_>,
    env: Env,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::InProgress)?;

    let threshold = THRESHOLD.load(deps.storage)?;
    if dealers_still_active(&deps)? < threshold as usize {
        reset_epoch_state(deps.storage)?;
        CURRENT_EPOCH.update::<_, ContractError>(deps.storage, |epoch| {
            Ok(Epoch::new(
                EpochState::default(),
                epoch.epoch_id + 1,
                epoch.time_configuration,
                env.block.time,
            ))
        })?;
    }

    Ok(Response::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::error::ContractError::EarlyEpochStateAdvancement;
    use crate::support::tests::fixtures::dealer_details_fixture;
    use crate::support::tests::helpers::{init_contract, GROUP_MEMBERS};
    use coconut_dkg_common::types::{
        ContractSafeBytes, DealerDetails, EpochState, TimeConfiguration,
    };
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Addr;
    use cw4::Member;
    use rusty_fork::rusty_fork_test;

    // Because of the global variable handling group, we need individual process for each test
    rusty_fork_test! {
        #[test]
        fn still_active() {
            let mut deps = init_contract();
            {
                let mut group = GROUP_MEMBERS.lock().unwrap();

                group.push(Member {
                    addr: "owner1".to_string(),
                    weight: 10,
                });
                group.push(Member {
                    addr: "owner2".to_string(),
                    weight: 10,
                });
                group.push(Member {
                    addr: "owner3".to_string(),
                    weight: 10,
                });
            }
            assert_eq!(0, dealers_still_active(&deps.as_mut()).unwrap());
            for i in 0..3 as u64 {
                let details = dealer_details_fixture(i + 1);
                current_dealers()
                    .save(deps.as_mut().storage, &details.address, &details)
                    .unwrap();
                assert_eq!(
                    i as usize + 1,
                    dealers_still_active(&deps.as_mut()).unwrap()
                );
            }
        }

        #[test]
        fn advance_state() {
            let mut deps = init_contract();
            let mut env = mock_env();
            {
                let mut group = GROUP_MEMBERS.lock().unwrap();

                group.push(Member {
                    addr: "owner1".to_string(),
                    weight: 10,
                });
                group.push(Member {
                    addr: "owner2".to_string(),
                    weight: 10,
                });
                group.push(Member {
                    addr: "owner3".to_string(),
                    weight: 10,
                });
            }

            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(epoch.state, EpochState::PublicKeySubmission{resharing: false});
            assert_eq!(
                epoch.finish_timestamp,
                env.block
                    .time
                    .plus_seconds(epoch.time_configuration.public_key_submission_time_secs)
            );

            env.block.time = env
                .block
                .time
                .plus_seconds(epoch.time_configuration.public_key_submission_time_secs - 1);
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(1)
            );

            env.block.time = env.block.time.plus_seconds(1);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(epoch.state, EpochState::DealingExchange{resharing: false});
            assert_eq!(
                epoch.finish_timestamp,
                env.block
                    .time
                    .plus_seconds(epoch.time_configuration.dealing_exchange_time_secs)
            );

            env.block.time = env
                .block
                .time
                .plus_seconds(epoch.time_configuration.dealing_exchange_time_secs - 2);
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(2)
            );

            env.block.time = env.block.time.plus_seconds(3);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(epoch.state, EpochState::VerificationKeySubmission{resharing: false});
            assert_eq!(
                epoch.finish_timestamp,
                env.block.time.plus_seconds(
                    epoch
                        .time_configuration
                        .verification_key_submission_time_secs
                )
            );

            env.block.time = env.block.time.plus_seconds(
                epoch
                    .time_configuration
                    .verification_key_submission_time_secs
                    - 2,
            );
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(2)
            );

            env.block.time = env.block.time.plus_seconds(3);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(epoch.state, EpochState::VerificationKeyValidation{resharing: false});
            assert_eq!(
                epoch.finish_timestamp,
                env.block.time.plus_seconds(
                    epoch
                        .time_configuration
                        .verification_key_validation_time_secs
                )
            );

            env.block.time = env.block.time.plus_seconds(
                epoch
                    .time_configuration
                    .verification_key_validation_time_secs
                    - 3,
            );
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(3)
            );

            env.block.time = env.block.time.plus_seconds(3);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(epoch.state, EpochState::VerificationKeyFinalization{resharing: false});
            assert_eq!(
                epoch.finish_timestamp,
                env.block.time.plus_seconds(
                    epoch
                        .time_configuration
                        .verification_key_finalization_time_secs
                )
            );

            env.block.time = env
                .block
                .time
                .plus_seconds(TimeConfiguration::default().verification_key_finalization_time_secs - 1);
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(1)
            );

            env.block.time = env.block.time.plus_seconds(1);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(epoch.state, EpochState::InProgress);
            assert_eq!(
                epoch.finish_timestamp,
                env.block
                    .time
                    .plus_seconds(epoch.time_configuration.in_progress_time_secs)
            );

            env.block.time = env
                .block
                .time
                .plus_seconds(epoch.time_configuration.in_progress_time_secs - 100);
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(100)
            );

            env.block.time = env.block.time.plus_seconds(50);
            assert_eq!(
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                EarlyEpochStateAdvancement(50)
            );

            // setup dealer details
            let all_details: [_; 3] = std::array::from_fn(|i| dealer_details_fixture(i as u64 + 1));
            for details in all_details.iter() {
                current_dealers()
                    .save(deps.as_mut().storage, &details.address, details)
                    .unwrap();
            }

            // Group hasn't changed, so we remain in the same epoch, with updated finish timestamp
            env.block.time = env.block.time.plus_seconds(100);
            let prev_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let curr_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            let expected_epoch = Epoch::new(
                EpochState::InProgress,
                prev_epoch.epoch_id,
                prev_epoch.time_configuration,
                env.block.time,
            );
            assert_eq!(curr_epoch, expected_epoch);

            // Group changed, slightly, so reset dkg state
            *GROUP_MEMBERS.lock().unwrap().first_mut().unwrap() = Member {
                addr: "owner4".to_string(),
                weight: 10,
            };
            env.block.time = env
                .block
                .time
                .plus_seconds(epoch.time_configuration.in_progress_time_secs);
            let prev_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let curr_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            let expected_epoch = Epoch::new(
                EpochState::default(),
                prev_epoch.epoch_id + 1,
                prev_epoch.time_configuration,
                env.block.time,
            );
            assert_eq!(curr_epoch, expected_epoch);
            assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
        }


    #[test]
    fn surpass_threshold() {
        let mut deps = init_contract();
        let mut env = mock_env();
        let time_configuration = TimeConfiguration::default();
        {
            let mut group = GROUP_MEMBERS.lock().unwrap();

            group.push(Member {
                addr: "owner1".to_string(),
                weight: 10,
            });
            group.push(Member {
                addr: "owner2".to_string(),
                weight: 10,
            });
            group.push(Member {
                addr: "owner3".to_string(),
                weight: 10,
            });
        }

        let ret = try_surpassed_threshold(deps.as_mut(), env.clone()).unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::default().to_string(),
                expected_state: EpochState::InProgress.to_string()
            }
        );

        let all_details: [_; 3] = std::array::from_fn(|i| dealer_details_fixture(i as u64 + 1));
        for details in all_details.iter() {
            current_dealers()
                .save(deps.as_mut().storage, &details.address, details)
                .unwrap();
        }

        for times in [
            time_configuration.public_key_submission_time_secs,
            time_configuration.dealing_exchange_time_secs,
            time_configuration.verification_key_submission_time_secs,
            time_configuration.verification_key_validation_time_secs,
            time_configuration.verification_key_finalization_time_secs,
        ] {
            env.block.time = env.block.time.plus_seconds(times);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        }
        let curr_epoch = CURRENT_EPOCH.load(&deps.storage).unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 2);

        // epoch hasn't advanced as we are still in the threshold range
        try_surpassed_threshold(deps.as_mut(), env.clone()).unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 2);
        assert_eq!(CURRENT_EPOCH.load(&deps.storage).unwrap(), curr_epoch);

        *GROUP_MEMBERS.lock().unwrap().first_mut().unwrap() = Member {
            addr: "owner4".to_string(),
            weight: 10,
        };
        // epoch hasn't advanced as we are still in the threshold range
        try_surpassed_threshold(deps.as_mut(), env.clone()).unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 2);
        assert_eq!(CURRENT_EPOCH.load(&deps.storage).unwrap(), curr_epoch);

        *GROUP_MEMBERS.lock().unwrap().last_mut().unwrap() = Member {
            addr: "owner5".to_string(),
            weight: 10,
        };
        try_surpassed_threshold(deps.as_mut(), env.clone()).unwrap();
        assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
        let next_epoch = CURRENT_EPOCH.load(&deps.storage).unwrap();
        assert_eq!(
            next_epoch,
            Epoch::new(
                EpochState::default(),
                curr_epoch.epoch_id + 1,
                curr_epoch.time_configuration,
                env.block.time,
            )
        );
    }
    }

    #[test]
    fn reset_state() {
        let mut deps = init_contract();
        let all_details: [_; 100] = std::array::from_fn(|i| dealer_details_fixture(i as u64));

        THRESHOLD.save(deps.as_mut().storage, &42).unwrap();
        for details in all_details.iter() {
            current_dealers()
                .save(deps.as_mut().storage, &details.address, details)
                .unwrap();
            for dealings in DEALINGS_BYTES {
                dealings
                    .save(
                        deps.as_mut().storage,
                        &details.address,
                        &ContractSafeBytes(vec![1, 2, 3]),
                    )
                    .unwrap();
            }
        }

        reset_epoch_state(deps.as_mut().storage).unwrap();

        assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
        for details in all_details {
            for dealings in DEALINGS_BYTES {
                assert!(dealings
                    .may_load(&deps.storage, &details.address)
                    .unwrap()
                    .is_none());
            }
            assert!(current_dealers()
                .may_load(deps.as_mut().storage, &details.address)
                .unwrap()
                .is_none());
            assert_eq!(
                past_dealers()
                    .load(&deps.storage, &details.address)
                    .unwrap(),
                details
            );
        }
    }

    #[test]
    fn verify_threshold() {
        let mut deps = init_contract();
        let mut env = mock_env();

        assert!(THRESHOLD.may_load(deps.as_mut().storage).unwrap().is_none());

        for i in 1..101 {
            let address = Addr::unchecked(format!("dealer{}", i));
            current_dealers()
                .save(
                    deps.as_mut().storage,
                    &address,
                    &DealerDetails {
                        address: address.clone(),
                        bte_public_key_with_proof: "bte_public_key_with_proof".to_string(),
                        announce_address: "127.0.0.1".to_string(),
                        assigned_index: i,
                    },
                )
                .unwrap();
        }

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        advance_epoch_state(deps.as_mut(), env).unwrap();
        assert_eq!(
            THRESHOLD.may_load(deps.as_mut().storage).unwrap().unwrap(),
            67
        );
    }
}
