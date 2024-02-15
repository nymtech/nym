// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::current_dealers;
use crate::epoch_state::storage::{CURRENT_EPOCH, INITIAL_REPLACEMENT_DATA, THRESHOLD};
use crate::epoch_state::transactions::{
    dealers_eq_members, replacement_threshold_surpassed, reset_dkg_state,
};
use crate::epoch_state::utils::{check_state_completion, needs_reset, needs_resharing};
use crate::error::ContractError;
use crate::state::storage::STATE;
use crate::verification_key_shares::storage::verified_dealers;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, Order, Response};
use nym_coconut_dkg_common::types::{Epoch, EpochState, InitialReplacementData};

fn ensure_can_advance_state(
    deps: Deps<'_>,
    env: &Env,
    current_epoch: &Epoch,
) -> Result<(), ContractError> {
    if current_epoch.state == EpochState::WaitingInitialisation {
        return Err(ContractError::WaitingInitialisation);
    }

    // check if we completed the state, so we could short circuit the deadline
    if check_state_completion(deps.storage, current_epoch)? {
        return Ok(());
    }

    // otherwise fallback to the deadline
    if let Some(finish_timestamp) = current_epoch.deadline {
        if finish_timestamp > env.block.time {
            return Err(ContractError::EarlyEpochStateAdvancement(
                finish_timestamp
                    .minus_seconds(env.block.time.seconds())
                    .seconds(),
            ));
        }
    }

    Ok(())
}

pub fn try_advance_epoch_state(deps: DepsMut<'_>, env: Env) -> Result<Response, ContractError> {
    // TODO: the only case where this can retrigger itself is when insufficient number of parties completed it, i.e. we don't have threshold

    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;

    // checks whether the given phase has either completed or reached its deadline
    ensure_can_advance_state(deps.as_ref(), &env, &current_epoch)?;

    let next_state = match current_epoch.state.next() {
        Some(next_state) => next_state,
        None => {
            debug_assert!(current_epoch.state.is_in_progress());
            // TODO: that's for the future because it will involve more changes in the other bits of the codebase
            // but change epoch_id upon extending time of the "in progress" phase and instead store a map of
            // [current_epoch_id => epoch_id_of_keys_creation] for key retrieval
            EpochState::InProgress
        }
    };

    // update the epoch state
    let mut next_epoch = current_epoch;
    next_epoch.state = next_state;
    CURRENT_EPOCH.save(deps.storage, &next_epoch)?;

    Ok(Response::new())

    //
    //
    // let next_epoch = if let Some(state) = current_epoch.state.next() {
    //     // We are during DKG process
    //     let mut new_state = state;
    //     if let EpochState::DealingExchange { .. } = state {
    //         let current_dealers = current_dealers()
    //             .keys(deps.storage, None, None, Order::Ascending)
    //             .collect::<Result<Vec<Addr>, _>>()?;
    //         let group_members =
    //             STATE
    //                 .load(deps.storage)?
    //                 .group_addr
    //                 .list_members(&deps.querier, None, None)?;
    //         if current_dealers.len() < group_members.len() {
    //             // If not all group members registered yet, we just stay in the same state until
    //             // they either register or they get kicked out of the group
    //             new_state = current_epoch.state;
    //         } else {
    //             // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
    //             let threshold = (2 * current_dealers.len() as u64 + 3 - 1) / 3;
    //             THRESHOLD.save(deps.storage, &threshold)?;
    //         }
    //     };
    //     Epoch::new(
    //         new_state,
    //         current_epoch.epoch_id,
    //         current_epoch.time_configuration,
    //         env.block.time,
    //     )
    // } else if dealers_eq_members(&deps)? {
    //     // The dealer set hasn't changed, so we only extend the finish timestamp
    //     // The epoch remains the same, as we use it as key for storing VKs
    //
    //     // TODO: change that behaviour in the following PR
    //     Epoch::new(
    //         current_epoch.state,
    //         current_epoch.epoch_id,
    //         current_epoch.time_configuration,
    //         env.block.time,
    //     )
    // } else {
    //     // Dealer set changed, we need to redo DKG...
    //     let state = if replacement_threshold_surpassed(&deps)? {
    //         // ... in reset mode
    //         INITIAL_REPLACEMENT_DATA.remove(deps.storage);
    //         EpochState::PublicKeySubmission { resharing: false }
    //     } else {
    //         // ... in reshare mode
    //         if INITIAL_REPLACEMENT_DATA.may_load(deps.storage)?.is_some() {
    //             INITIAL_REPLACEMENT_DATA.update::<_, ContractError>(deps.storage, |mut data| {
    //                 // TODO: FIXME: for second reshare the added set of dealers won't be allowed to participate
    //                 data.initial_height = env.block.height;
    //                 Ok(data)
    //             })?;
    //         } else {
    //             let replacement_data = InitialReplacementData {
    //                 initial_dealers: verified_dealers(deps.storage)?,
    //                 initial_height: env.block.height,
    //             };
    //             INITIAL_REPLACEMENT_DATA.save(deps.storage, &replacement_data)?;
    //         }
    //
    //         EpochState::PublicKeySubmission { resharing: true }
    //     };
    //     reset_dkg_state(deps.storage)?;
    //     Epoch::new(
    //         state,
    //         current_epoch.epoch_id + 1,
    //         current_epoch.time_configuration,
    //         env.block.time,
    //     )
    // };
    // CURRENT_EPOCH.save(deps.storage, &next_epoch)?;
    //
    // Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dealers::storage::past_dealers;
    use crate::epoch_state::transactions::try_initiate_dkg;
    use crate::error::ContractError::EarlyEpochStateAdvancement;
    use crate::support::tests::fixtures::{dealer_details_fixture, vk_share_fixture};
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS, GROUP_MEMBERS};
    use crate::verification_key_shares::storage::vk_shares;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cw4::Member;
    use nym_coconut_dkg_common::types::TimeConfiguration;

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

        // can't advance the state if dkg hasn't been initiated
        assert_eq!(
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            ContractError::WaitingInitialisation
        );

        try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        assert_eq!(
            epoch.deadline.unwrap(),
            env.block
                .time
                .plus_seconds(epoch.time_configuration.public_key_submission_time_secs)
        );

        env.block.time = env
            .block
            .time
            .plus_seconds(epoch.time_configuration.public_key_submission_time_secs - 1);
        assert_eq!(
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(1)
        );

        env.block.time = env.block.time.plus_seconds(1);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );

        // setup dealer details
        let all_shares: [_; 4] =
            std::array::from_fn(|i| vk_share_fixture(&format!("owner{}", i + 1), 0));
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
        env.block.time = env
            .block
            .time
            .plus_seconds(epoch.time_configuration.public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::DealingExchange { resharing: false }
        );
        assert_eq!(
            epoch.deadline.unwrap(),
            env.block
                .time
                .plus_seconds(epoch.time_configuration.dealing_exchange_time_secs)
        );

        env.block.time = env
            .block
            .time
            .plus_seconds(epoch.time_configuration.dealing_exchange_time_secs - 2);
        assert_eq!(
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(2)
        );

        env.block.time = env.block.time.plus_seconds(3);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::VerificationKeySubmission { resharing: false }
        );
        assert_eq!(
            epoch.deadline.unwrap(),
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
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(2)
        );

        env.block.time = env.block.time.plus_seconds(3);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::VerificationKeyValidation { resharing: false }
        );
        assert_eq!(
            epoch.deadline.unwrap(),
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
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(3)
        );

        env.block.time = env.block.time.plus_seconds(3);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::VerificationKeyFinalization { resharing: false }
        );
        assert_eq!(
            epoch.deadline.unwrap(),
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
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(1)
        );

        env.block.time = env.block.time.plus_seconds(1);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::InProgress);
        assert_eq!(
            epoch.deadline.unwrap(),
            env.block
                .time
                .plus_seconds(epoch.time_configuration.in_progress_time_secs)
        );

        env.block.time = env
            .block
            .time
            .plus_seconds(epoch.time_configuration.in_progress_time_secs - 100);
        assert_eq!(
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(100)
        );

        env.block.time = env.block.time.plus_seconds(50);
        assert_eq!(
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            EarlyEpochStateAdvancement(50)
        );

        // Group hasn't changed, so we remain in the same epoch, with updated finish timestamp
        env.block.time = env.block.time.plus_seconds(100);
        let prev_epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
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
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
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
            past_dealers()
                .remove(deps.as_mut().storage, &details.address)
                .unwrap();
            current_dealers()
                .save(deps.as_mut().storage, &details.address, details)
                .unwrap();
        }
        for times in [
            epoch.time_configuration.public_key_submission_time_secs,
            epoch.time_configuration.dealing_exchange_time_secs,
            epoch
                .time_configuration
                .verification_key_submission_time_secs,
            epoch
                .time_configuration
                .verification_key_validation_time_secs,
            epoch
                .time_configuration
                .verification_key_finalization_time_secs,
        ] {
            env.block.time = env.block.time.plus_seconds(times);
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
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
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
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
}
