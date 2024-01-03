// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{current_dealers, past_dealers};
use crate::epoch_state::storage::{CURRENT_EPOCH, INITIAL_REPLACEMENT_DATA, THRESHOLD};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::STATE;
use crate::verification_key_shares::storage::verified_dealers;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, Order, Response, Storage};
use nym_coconut_dkg_common::types::{Epoch, EpochState, InitialReplacementData};

fn reset_epoch_state(storage: &mut dyn Storage) -> Result<(), ContractError> {
    THRESHOLD.remove(storage);
    let dealers: Vec<_> = current_dealers()
        .keys(storage, None, None, Order::Ascending)
        .collect::<Result<_, _>>()?;

    for dealer_addr in dealers {
        let details = current_dealers().load(storage, &dealer_addr)?;
        current_dealers().remove(storage, &dealer_addr)?;
        past_dealers().save(storage, &dealer_addr, &details)?;
    }
    Ok(())
}

fn dealers_still_active(
    deps: &Deps<'_>,
    dealers: impl Iterator<Item = Addr>,
) -> Result<usize, ContractError> {
    let state = STATE.load(deps.storage)?;
    let mut still_active = 0;
    for dealer_addr in dealers {
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
    let verified_dealers = verified_dealers(deps.storage)?;
    let all_dealers = verified_dealers.len();
    let dealers_still_active = dealers_still_active(&deps.as_ref(), verified_dealers.into_iter())?;
    let group_members = STATE
        .load(deps.storage)?
        .group_addr
        .list_members(&deps.querier, None, None)?
        .len();

    Ok(dealers_still_active == all_dealers && all_dealers == group_members)
}

fn replacement_threshold_surpassed(deps: &DepsMut<'_>) -> Result<bool, ContractError> {
    let threshold = THRESHOLD.load(deps.storage)? as usize;
    let initial_dealers = verified_dealers(deps.storage)?;
    if initial_dealers.is_empty() {
        // possibly failed DKG, just reset and start again
        return Ok(true);
    }
    let initial_dealer_count = initial_dealers.len();
    let replacement_threshold = threshold - (initial_dealers.len() + 2 - 1) / 2 + 1;
    let removed_dealer_count =
        initial_dealer_count - dealers_still_active(&deps.as_ref(), initial_dealers.into_iter())?;

    Ok(removed_dealer_count >= replacement_threshold)
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
        let mut new_state = state;
        if let EpochState::DealingExchange { .. } = state {
            let current_dealers = current_dealers()
                .keys(deps.storage, None, None, Order::Ascending)
                .collect::<Result<Vec<Addr>, _>>()?;
            let group_members =
                STATE
                    .load(deps.storage)?
                    .group_addr
                    .list_members(&deps.querier, None, None)?;
            if current_dealers.len() < group_members.len() {
                // If not all group members registered yet, we just stay in the same state until
                // they either register or they get kicked out of the group
                new_state = current_epoch.state;
            } else {
                // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
                let threshold = (2 * current_dealers.len() as u64 + 3 - 1) / 3;
                THRESHOLD.save(deps.storage, &threshold)?;
            }
        };
        Epoch::new(
            new_state,
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
        // Dealer set changed, we need to redo DKG...
        let state = if replacement_threshold_surpassed(&deps)? {
            // ... in reset mode
            INITIAL_REPLACEMENT_DATA.remove(deps.storage);
            EpochState::default()
        } else {
            // ... in reshare mode
            if INITIAL_REPLACEMENT_DATA.may_load(deps.storage)?.is_some() {
                INITIAL_REPLACEMENT_DATA.update::<_, ContractError>(deps.storage, |mut data| {
                    data.initial_height = env.block.height;
                    Ok(data)
                })?;
            } else {
                let replacement_data = InitialReplacementData {
                    initial_dealers: verified_dealers(deps.storage)?,
                    initial_height: env.block.height,
                };
                INITIAL_REPLACEMENT_DATA.save(deps.storage, &replacement_data)?;
            }

            EpochState::PublicKeySubmission { resharing: true }
        };
        reset_epoch_state(deps.storage)?;
        Epoch::new(
            state,
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
    let dealers = verified_dealers(deps.storage)?;
    if dealers_still_active(&deps.as_ref(), dealers.into_iter())? < threshold as usize {
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
    use crate::support::tests::fixtures::{dealer_details_fixture, vk_share_fixture};
    use crate::support::tests::helpers::{init_contract, GROUP_MEMBERS};
    use crate::verification_key_shares::storage::vk_shares;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Addr;
    use cw4::Member;
    use nym_coconut_dkg_common::types::{DealerDetails, EpochState, TimeConfiguration};
    use rusty_fork::rusty_fork_test;

    // Because of the global variable handling group, we need individual process for each test

    rusty_fork_test! {
        // Using values from the DKG document
        #[test]
        fn threshold_surpassed() {
            let mut deps = init_contract();
            let two_thirds = |n: u64| (2 * n + 3 - 1) / 3;
            let three_fourths = |n: u64| (3 * n + 4 - 1) / 4;
            let ninty_pc = |n: u64| (9 * n + 10 - 2) / 10;
            let mut limits = [3, 4, 5, 5, 7, 11, 10, 14, 21, 18, 26, 41].iter();

            for n in [10, 25, 50, 100] {
                let dealers: Vec<_> = (0..n).map(dealer_details_fixture).collect();
                let shares: Vec<_> = (0..n).map(|idx| vk_share_fixture(&format!("owner{}", idx), 0)).collect();
                let initial_dealers = dealers.iter().map(|d| d.address.clone()).collect();
                let data = InitialReplacementData {
                    initial_dealers,
                    initial_height: 1,
                };
                for share in shares {
                    vk_shares().save(deps.as_mut().storage, (&share.owner, 0), &share).unwrap();
                }
                for f in [two_thirds, three_fourths, ninty_pc] {
                    let threshold = f(n);
                    THRESHOLD.save(deps.as_mut().storage, &threshold).unwrap();
                    INITIAL_REPLACEMENT_DATA
                        .save(deps.as_mut().storage, &data)
                        .unwrap();

                    let limit = *limits.next().unwrap();
                    {
                        let mut group_members = GROUP_MEMBERS.lock().unwrap();
                        for dealer in dealers.iter() {
                            group_members.push((
                                Member {
                                    addr: dealer.address.to_string(),
                                    weight: 10,
                                },
                                1,
                            ));
                        }
                        for _ in 1..limit {
                            group_members.pop();
                        }
                    }
                    assert!(!replacement_threshold_surpassed(&deps.as_mut()).unwrap());
                    GROUP_MEMBERS.lock().unwrap().pop();
                    assert!(replacement_threshold_surpassed(&deps.as_mut()).unwrap());

                    *GROUP_MEMBERS.lock().unwrap() = vec![];
                }
            }
        }

        #[test]
        fn dealers_and_members() {
            let mut deps = init_contract();

            assert!(dealers_eq_members(&deps.as_mut()).unwrap());

            let share = vk_share_fixture("owner2", 0);
            let different_share = vk_share_fixture("owner4", 0);
            vk_shares()
                .save(deps.as_mut().storage, (&share.owner, 0), &share)
                .unwrap();
            assert!(!dealers_eq_members(&deps.as_mut()).unwrap());

            vk_shares()
                .remove(deps.as_mut().storage, (&share.owner, 0))
                .unwrap();
            GROUP_MEMBERS.lock().unwrap().push((
                Member {
                    addr: "owner2".to_string(),
                    weight: 10,
                },
                1,
            ));
            assert!(!dealers_eq_members(&deps.as_mut()).unwrap());

            vk_shares()
                .save(
                    deps.as_mut().storage,
                    (&different_share.owner, 0),
                    &different_share,
                )
                .unwrap();
            assert!(!dealers_eq_members(&deps.as_mut()).unwrap());

            vk_shares()
                .remove(deps.as_mut().storage, (&different_share.owner, 0))
                .unwrap();
            vk_shares()
                .save(deps.as_mut().storage, (&share.owner, 0), &share)
                .unwrap();
            assert!(dealers_eq_members(&deps.as_mut()).unwrap());
        }

        #[test]
        fn still_active() {
            let mut deps = init_contract();
            {
                let mut group = GROUP_MEMBERS.lock().unwrap();

                group.push((
                    Member {
                        addr: "owner1".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner2".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner3".to_string(),
                        weight: 10,
                    },
                    1,
                ));
            }
            assert_eq!(
                0,
                dealers_still_active(
                    &deps.as_ref(),
                    current_dealers()
                        .keys(&deps.storage, None, None, Order::Ascending)
                        .flatten()
                )
                .unwrap()
            );
            for i in 0..3_u64 {
                let details = dealer_details_fixture(i + 1);
                current_dealers()
                    .save(deps.as_mut().storage, &details.address, &details)
                    .unwrap();
                assert_eq!(
                    i as usize + 1,
                    dealers_still_active(
                        &deps.as_ref(),
                        current_dealers()
                            .keys(&deps.storage, None, None, Order::Ascending)
                            .flatten()
                    )
                    .unwrap()
                );
            }
        }

        #[test]
        fn advance_state() {
            let mut deps = init_contract();
            let mut env = mock_env();
            {
                let mut group = GROUP_MEMBERS.lock().unwrap();

                group.push((
                    Member {
                        addr: "owner1".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner2".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner3".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner4".to_string(),
                        weight: 10,
                    },
                    1,
                ));
            }

            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(
                epoch.state,
                EpochState::PublicKeySubmission { resharing: false }
            );
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
            assert_eq!(
                epoch.state,
                EpochState::PublicKeySubmission { resharing: false }
            );

            // setup dealer details
            let all_shares: [_; 4] = std::array::from_fn(|i| vk_share_fixture(&format!("owner{}", i + 1), 0));
            for share in all_shares.iter() {
                vk_shares()
                    .save(deps.as_mut().storage, (&share.owner, 0), share)
                    .unwrap();
            }
            let all_details: [_; 4] = std::array::from_fn(|i| dealer_details_fixture(i as u64 + 1));
            for details in all_details.iter() {
                current_dealers()
                    .save(deps.as_mut().storage, &details.address, details)
                    .unwrap();
            }

            assert!(INITIAL_REPLACEMENT_DATA
                .may_load(&deps.storage)
                .unwrap()
                .is_none());
            env.block.time = env.block.time.plus_seconds(epoch.time_configuration.public_key_submission_time_secs);
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            assert_eq!(
                epoch.state,
                EpochState::DealingExchange { resharing: false }
            );
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
            assert_eq!(
                epoch.state,
                EpochState::VerificationKeySubmission { resharing: false }
            );
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
            assert_eq!(
                epoch.state,
                EpochState::VerificationKeyValidation { resharing: false }
            );
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
            assert_eq!(
                epoch.state,
                EpochState::VerificationKeyFinalization { resharing: false }
            );
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

            // Group changed slightly, so re-run dkg in reshare mode
            *GROUP_MEMBERS.lock().unwrap().first_mut().unwrap() = (
                Member {
                    addr: "owner5".to_string(),
                    weight: 10,
                },
                1,
            );
            env.block.time = env
                .block
                .time
                .plus_seconds(epoch.time_configuration.in_progress_time_secs);
            let prev_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let curr_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            let expected_epoch = Epoch::new(
                EpochState::PublicKeySubmission { resharing: true },
                prev_epoch.epoch_id + 1,
                prev_epoch.time_configuration,
                env.block.time,
            );
            assert_eq!(curr_epoch, expected_epoch);
            assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
            let replacement_data = INITIAL_REPLACEMENT_DATA.load(&deps.storage).unwrap();
            let expected_replacement_data = InitialReplacementData {
                initial_dealers: all_details.iter().map(|d| d.address.clone()).collect(),
                initial_height: 12345,
            };
            assert_eq!(replacement_data, expected_replacement_data);

            let all_details: [_; 4] = std::array::from_fn(|i| dealer_details_fixture(i as u64 + 2));
            for details in all_details.iter() {
                past_dealers().remove(deps.as_mut().storage, &details.address).unwrap();
                current_dealers()
                    .save(deps.as_mut().storage, &details.address, details)
                    .unwrap();
            }
            for times in [
                epoch.time_configuration.public_key_submission_time_secs,
                epoch.time_configuration.dealing_exchange_time_secs,
                epoch.time_configuration.verification_key_submission_time_secs,
                epoch.time_configuration.verification_key_validation_time_secs,
                epoch.time_configuration.verification_key_finalization_time_secs,
            ] {
                env.block.time = env.block.time.plus_seconds(times);
                advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            }

            let all_shares: [_; 4] = std::array::from_fn(|i| {
                let mut share = vk_share_fixture(&format!("owner{}", i + 1), 1);
                share.verified = i % 2 == 0;
                share
                });
            for share in all_shares.iter() {
                vk_shares()
                    .save(deps.as_mut().storage, (&share.owner, 0), share)
                    .unwrap();
            }

            // Group changed even more, surpassing threshold, so re-run dkg in reset mode
            *GROUP_MEMBERS.lock().unwrap().last_mut().unwrap() = (
                Member {
                    addr: "owner6".to_string(),
                    weight: 10,
                },
                1,
            );
            env.block.time = env
                .block
                .time
                .plus_seconds(epoch.time_configuration.in_progress_time_secs);
            let prev_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
            let curr_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
            let expected_epoch = Epoch::new(
                EpochState::PublicKeySubmission { resharing: true },
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

                group.push((
                    Member {
                        addr: "owner1".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner2".to_string(),
                        weight: 10,
                    },
                    1,
                ));
                group.push((
                    Member {
                        addr: "owner3".to_string(),
                        weight: 10,
                    },
                    1,
                ));
            }

            let ret = try_surpassed_threshold(deps.as_mut(), env.clone()).unwrap_err();
            assert_eq!(
                ret,
                ContractError::IncorrectEpochState {
                    current_state: EpochState::default().to_string(),
                    expected_state: EpochState::InProgress.to_string()
                }
            );


            let all_shares: [_; 3] = std::array::from_fn(|i| vk_share_fixture(&format!("owner{}", i + 1), 0));
            for share in all_shares.iter() {
                vk_shares()
                    .save(deps.as_mut().storage, (&share.owner, 0), share)
                    .unwrap();
            }
            let all_details: [_; 3] = std::array::from_fn(|i| dealer_details_fixture(i as u64 + 1));
            for details in all_details.iter() {
                current_dealers()
                    .save(deps.as_mut().storage, &details.address, details)
                    .unwrap();
            }
            let all_shares: [_; 3] = std::array::from_fn(|i| vk_share_fixture(&format!("owner{}", i + 1), 0));
            for share in all_shares.iter() {
                vk_shares()
                    .save(deps.as_mut().storage, (&share.owner, share.epoch_id), share)
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

            *GROUP_MEMBERS.lock().unwrap().first_mut().unwrap() = (
                Member {
                    addr: "owner4".to_string(),
                    weight: 10,
                },
                1,
            );
            // epoch hasn't advanced as we are still in the threshold range
            try_surpassed_threshold(deps.as_mut(), env.clone()).unwrap();
            assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 2);
            assert_eq!(CURRENT_EPOCH.load(&deps.storage).unwrap(), curr_epoch);

            *GROUP_MEMBERS.lock().unwrap().last_mut().unwrap() = (
                Member {
                    addr: "owner5".to_string(),
                    weight: 10,
                },
                1,
            );
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
        }

        reset_epoch_state(deps.as_mut().storage).unwrap();

        assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
        for details in all_details {
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
