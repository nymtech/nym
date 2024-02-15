// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{current_dealers, past_dealers};
use crate::epoch_state::storage::{CURRENT_EPOCH, THRESHOLD};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::{DKG_ADMIN, STATE};
use crate::verification_key_shares::storage::verified_dealers;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Order, Response, Storage};
use cw4::Member;
use nym_coconut_dkg_common::types::{Epoch, EpochState};

pub use advance_epoch_state::try_advance_epoch_state;

pub mod advance_epoch_state;

fn reset_dkg_state(storage: &mut dyn Storage) -> Result<(), ContractError> {
    THRESHOLD.remove(storage);

    // TODO: unbounded query. to be removed be preserving dealer details and changing storage structure
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

#[deprecated]
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

pub(crate) fn try_initiate_dkg(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to kick start the process
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let epoch = CURRENT_EPOCH.load(deps.storage)?;
    if !matches!(epoch.state, EpochState::WaitingInitialisation) {
        return Err(ContractError::AlreadyInitialised);
    }

    // the first exchange won't involve resharing
    let initial_state = EpochState::PublicKeySubmission { resharing: false };
    let initial_epoch = Epoch::new(initial_state, 0, epoch.time_configuration, env.block.time);
    CURRENT_EPOCH.save(deps.storage, &initial_epoch)?;

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
        reset_dkg_state(deps.storage)?;
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

pub(crate) fn try_trigger_reset(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to trigger DKG reset
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;

    // only allow reset when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = Epoch::new(
        EpochState::PublicKeySubmission { resharing: false },
        current_epoch.epoch_id + 1,
        current_epoch.time_configuration,
        env.block.time,
    );
    CURRENT_EPOCH.save(deps.storage, &next_epoch)?;

    // TODO: remove
    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

pub(crate) fn try_trigger_resharing(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to trigger DKG resharing
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;

    // only allow resharing when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = Epoch::new(
        EpochState::PublicKeySubmission { resharing: true },
        current_epoch.epoch_id + 1,
        current_epoch.time_configuration,
        env.block.time,
    );
    CURRENT_EPOCH.save(deps.storage, &next_epoch)?;

    // TODO: remove
    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::storage::INITIAL_REPLACEMENT_DATA;
    use crate::epoch_state::transactions::advance_epoch_state::try_advance_epoch_state;
    use crate::support::tests::fixtures::{dealer_details_fixture, vk_share_fixture};
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS, GROUP_MEMBERS};
    use crate::verification_key_shares::storage::vk_shares;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;
    use cw4::Member;
    use cw_controllers::AdminError;
    use nym_coconut_dkg_common::types::{
        DealerDetails, EpochState, InitialReplacementData, TimeConfiguration,
    };
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
                let shares: Vec<_> = (0..n)
                    .map(|idx| vk_share_fixture(&format!("owner{}", idx), 0))
                    .collect();
                let initial_dealers = dealers.iter().map(|d| d.address.clone()).collect();
                let data = InitialReplacementData {
                    initial_dealers,
                    initial_height: 1,
                };
                for share in shares {
                    vk_shares()
                        .save(deps.as_mut().storage, (&share.owner, 0), &share)
                        .unwrap();
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
        fn surpass_threshold() {
            let mut deps = init_contract();
            let mut env = mock_env();
            try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

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
                    current_state: EpochState::PublicKeySubmission { resharing: false }.to_string(),
                    expected_state: EpochState::InProgress.to_string()
                }
            );

            let all_shares: [_; 3] =
                std::array::from_fn(|i| vk_share_fixture(&format!("owner{}", i + 1), 0));
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
            let all_shares: [_; 3] =
                std::array::from_fn(|i| vk_share_fixture(&format!("owner{}", i + 1), 0));
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
                try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
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
    fn initialising_dkg() {
        let mut deps = init_contract();
        let env = mock_env();

        let initial_epoch_info = CURRENT_EPOCH.load(&deps.storage).unwrap();
        assert!(initial_epoch_info.deadline.is_none());

        // can only be executed by the admin
        let res = try_initiate_dkg(deps.as_mut(), env.clone(), mock_info("not an admin", &[]))
            .unwrap_err();
        assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);

        let res = try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[]));
        assert!(res.is_ok());

        // can't be initialised more than once
        let res = try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[]))
            .unwrap_err();
        assert_eq!(ContractError::AlreadyInitialised, res);

        // sets the correct epoch data
        let epoch = CURRENT_EPOCH.load(&deps.storage).unwrap();
        assert_eq!(epoch.epoch_id, 0);
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        assert_eq!(
            epoch.time_configuration,
            initial_epoch_info.time_configuration
        );
        assert_eq!(
            epoch.deadline.unwrap(),
            env.block
                .time
                .plus_seconds(epoch.time_configuration.public_key_submission_time_secs)
        );
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

        reset_dkg_state(deps.as_mut().storage).unwrap();

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
        try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

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
                        ed25519_identity: "identity".to_string(),
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
        try_advance_epoch_state(deps.as_mut(), env).unwrap();
        assert_eq!(
            THRESHOLD.may_load(deps.as_mut().storage).unwrap().unwrap(),
            67
        );
    }
}
