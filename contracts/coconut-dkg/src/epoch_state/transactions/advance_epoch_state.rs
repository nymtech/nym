// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{load_current_epoch, save_epoch, EPOCH_THRESHOLDS, THRESHOLD};
use crate::epoch_state::transactions::reset_dkg_state;
use crate::epoch_state::utils::check_state_completion;
use crate::error::ContractError;
use cosmwasm_std::{Deps, DepsMut, Env, Response};
use nym_coconut_dkg_common::types::{Epoch, EpochState};

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

    let current_epoch = load_current_epoch(deps.storage)?;

    // checks whether the given phase has either completed or reached its deadline
    ensure_can_advance_state(deps.as_ref(), &env, &current_epoch)?;

    // a ceremony can't start without dealers. the threshold would be `ceil(2 * 0 / 3)`, i.e.
    // zero, and every subsequent phase would be trivially complete (no dealings to wait for,
    // no shares to verify), so the epoch would run itself to the end and settle in progress
    // with no signers at all - and stay there, since that same vacuous comparison gates every
    // later advance too.
    //
    // so hold here instead, with a fresh window rather than an expired one: registration stays
    // open the whole time, and whoever comes back first still gets the full submission period
    // for the others to join rather than being able to advance alone the moment it registers
    if matches!(current_epoch.state, EpochState::PublicKeySubmission { .. })
        && current_epoch.state_progress.registered_dealers == 0
    {
        let current_state = current_epoch.state;
        let extended = current_epoch.update(current_state, env.block.time);
        save_epoch(deps.storage, env.block.height, &extended)?;
        return Ok(Response::new());
    }

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

    // if we're advancing into dealing exchange, we need to set the threshold value based on the number of registered dealers
    if next_state.is_dealing_exchange() {
        let registered_dealers = current_epoch.state_progress.registered_dealers as u64;
        // set the threshold to 2/3 amount of registered dealers
        let threshold = (2 * registered_dealers).div_ceil(3);

        // update current threshold values
        THRESHOLD.save(deps.storage, &threshold)?;
        EPOCH_THRESHOLDS.save(deps.storage, current_epoch.epoch_id, &threshold)?;
    }

    // edge case: we have completed DKG with fewer than threshold number of verified keys.
    // we have no choice but to reset since no credentials can be issued anyway.
    // TODO: is this actually a desired behaviour?
    let next_epoch = if next_state.is_in_progress() {
        let threshold = THRESHOLD.load(deps.storage)?;
        if (current_epoch.state_progress.verified_keys as u64) < threshold {
            reset_dkg_state(deps.storage)?;
            current_epoch.next_reset(env.block.time)
        } else {
            current_epoch.update(next_state, env.block.time)
        }
    } else {
        current_epoch.update(next_state, env.block.time)
    };

    // update the epoch state
    save_epoch(deps.storage, env.block.height, &next_epoch)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_state::storage::load_current_epoch;
    use crate::epoch_state::transactions::try_initiate_dkg;
    use crate::epoch_state::utils::check_epoch_state;
    use crate::error::ContractError::EarlyEpochStateAdvancement;
    use crate::state::storage::STATE;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::{Addr, Storage};
    use nym_coconut_dkg_common::types::TimeConfiguration;

    fn update_epoch<A>(storage: &mut dyn Storage, env: &Env, action: A)
    where
        A: Fn(Epoch) -> Epoch,
    {
        let current = load_current_epoch(storage).unwrap();
        let updated = action(current);
        save_epoch(storage, env.block.height, &updated).unwrap();
    }

    #[test]
    fn short_circuit_advance_state() {
        fn epoch_in_state(state: EpochState, env: &Env) -> Epoch {
            Epoch::new(state, 0, Default::default(), env.block.time)
        }

        fn set_epoch(storage: &mut dyn Storage, env: &Env, epoch: Epoch) {
            save_epoch(storage, env.block.height, &epoch).unwrap();
        }

        let mut deps = init_contract();
        let env = mock_env();

        // it's never possible to short-circuit `WaitingInitialisation`
        let epoch = epoch_in_state(EpochState::WaitingInitialisation, &env);
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        // neither PublicKeySubmission (in either resharing or non-resharing)
        let epoch = epoch_in_state(EpochState::PublicKeySubmission { resharing: false }, &env);
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let epoch = epoch_in_state(EpochState::PublicKeySubmission { resharing: true }, &env);
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let key_size = STATE.load(&deps.storage).unwrap().key_size;

        THRESHOLD.save(deps.as_mut().storage, &3).unwrap();

        // we can short-circuit `DealingExchange` if all dealers submitted their dealings

        // no dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false }, &env);
        epoch.state_progress.registered_dealers = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        // some dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false }, &env);
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_dealings = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        // all dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false }, &env);
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_dealings = key_size * 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_ok());
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::VerificationKeySubmission { resharing: false },
        )
        .unwrap();

        // no dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true }, &env);
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        // some dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true }, &env);
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        epoch.state_progress.submitted_dealings = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        // all dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true }, &env);
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        epoch.state_progress.submitted_dealings = key_size * 4;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_ok());
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::VerificationKeySubmission { resharing: true },
        )
        .unwrap();

        // we can short-circuit `VerificationKeySubmission` if all dealers submitted their verification keys
        let mut epoch = epoch_in_state(
            EpochState::VerificationKeySubmission { resharing: false },
            &env,
        );
        epoch.state_progress.registered_dealers = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeySubmission { resharing: true },
            &env,
        );
        epoch.state_progress.registered_dealers = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeySubmission { resharing: false },
            &env,
        );
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 4;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeySubmission { resharing: true },
            &env,
        );
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 4;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeySubmission { resharing: false },
            &env,
        );
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_ok());
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::VerificationKeyValidation { resharing: false },
        )
        .unwrap();

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeySubmission { resharing: true },
            &env,
        );
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_ok());
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::VerificationKeyValidation { resharing: true },
        )
        .unwrap();

        // can't short-circuit `VerificationKeyValidation` => we rely on multisig votes here
        let epoch = epoch_in_state(
            EpochState::VerificationKeyValidation { resharing: false },
            &env,
        );
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let epoch = epoch_in_state(
            EpochState::VerificationKeyValidation { resharing: true },
            &env,
        );
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        // we can short-circuit `VerificationKeyFinalization` if all submitted keys got verified
        let mut epoch = epoch_in_state(
            EpochState::VerificationKeyFinalization { resharing: false },
            &env,
        );
        epoch.state_progress.submitted_key_shares = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeyFinalization { resharing: true },
            &env,
        );
        epoch.state_progress.submitted_key_shares = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeyFinalization { resharing: false },
            &env,
        );
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 4;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeyFinalization { resharing: true },
            &env,
        );
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 4;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeyFinalization { resharing: false },
            &env,
        );
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_ok());
        check_epoch_state(deps.as_ref().storage, EpochState::InProgress).unwrap();

        let mut epoch = epoch_in_state(
            EpochState::VerificationKeyFinalization { resharing: true },
            &env,
        );
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 5;
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_ok());
        check_epoch_state(deps.as_ref().storage, EpochState::InProgress).unwrap();

        // it's never possible to short-circuit `InProgress`
        let epoch = epoch_in_state(EpochState::InProgress, &env);
        set_epoch(deps.as_mut().storage, &env, epoch);
        let res = try_advance_epoch_state(deps.as_mut(), env.clone());
        assert!(res.is_err());
    }

    #[test]
    fn advance_state_with_deadline() {
        let mut deps = init_contract();
        let mut env = mock_env();

        // can't advance the state if dkg hasn't been initiated
        assert_eq!(
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
            ContractError::WaitingInitialisation
        );

        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        let epoch = load_current_epoch(deps.as_mut().storage).unwrap();
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

        // add some dealers to prevent short-circuiting
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 42;
            e
        });
        env.block.time = env
            .block
            .time
            .plus_seconds(epoch.time_configuration.public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = load_current_epoch(deps.as_mut().storage).unwrap();
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
        let epoch = load_current_epoch(deps.as_mut().storage).unwrap();
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
        let epoch = load_current_epoch(deps.as_mut().storage).unwrap();
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

        // add some key shares to prevent short-circuiting
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.submitted_key_shares = 42;
            e
        });
        env.block.time = env.block.time.plus_seconds(3);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = load_current_epoch(deps.as_mut().storage).unwrap();
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

        // add some finalized keys to prevent reset
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.verified_keys = 42;
            e
        });

        env.block.time = env.block.time.plus_seconds(1);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let epoch = load_current_epoch(deps.as_mut().storage).unwrap();
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
        let prev_epoch = load_current_epoch(deps.as_mut().storage).unwrap();
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let curr_epoch = load_current_epoch(deps.as_mut().storage).unwrap();
        let mut expected_epoch = Epoch::new(
            EpochState::InProgress,
            prev_epoch.epoch_id,
            prev_epoch.time_configuration,
            env.block.time,
        );
        expected_epoch.state_progress = curr_epoch.state_progress;
        assert_eq!(curr_epoch, expected_epoch);

        // advancing from key finalization without threshold keys verified results in reset
        THRESHOLD.save(deps.as_mut().storage, &42).unwrap();
        let mut epoch = Epoch::new(
            EpochState::VerificationKeyFinalization { resharing: true },
            10,
            TimeConfiguration::default(),
            env.block.time,
        );

        // fewer than the threshold
        epoch.state_progress.verified_keys = 41;
        save_epoch(deps.as_mut().storage, env.block.height, &epoch).unwrap();
        env.block.time = env.block.time.plus_seconds(5000000);

        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let curr_epoch = load_current_epoch(deps.as_mut().storage).unwrap();
        let expected_epoch = Epoch::new(
            EpochState::PublicKeySubmission { resharing: false },
            epoch.epoch_id + 1,
            epoch.time_configuration,
            env.block.time,
        );
        assert_eq!(curr_epoch, expected_epoch);
        assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
    }

    /// A ceremony nobody took part in must not start, let alone conclude.
    ///
    /// Nothing here is set by hand: with no dealers every phase after this one would be
    /// trivially "complete" (zero of zero dealings submitted, zero of zero shares verified),
    /// so left to itself the ceremony would run all the way to the end and settle in progress
    /// with no signers at all.
    #[test]
    fn a_ceremony_nobody_joined_never_leaves_public_key_submission() {
        let mut deps = init_contract();
        let mut env = mock_env();

        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();
        let initial_epoch_id = load_current_epoch(&deps.storage).unwrap().epoch_id;

        // every api is down, say, so not a single dealer registers. one jump per phase the
        // ceremony would otherwise have walked through, each longer than the longest of them
        for _ in 0..5 {
            env.block.time = env.block.time.plus_seconds(601);
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

            let epoch = load_current_epoch(&deps.storage).unwrap();
            assert_eq!(
                epoch.state,
                EpochState::PublicKeySubmission { resharing: false },
                "a ceremony with no dealers at all started anyway"
            );
            // no epoch id is burned while waiting
            assert_eq!(epoch.epoch_id, initial_epoch_id);
            // and the wait is spent with an open registration window rather than an expired one
            assert_eq!(
                epoch.deadline.unwrap(),
                env.block
                    .time
                    .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs)
            );
        }
    }

    /// The hold is on having *nobody*, not on having too few: a single dealer is enough to
    /// start, and the resulting one-of-one threshold is deliberate (a testnet with one api
    /// should still be able to issue).
    #[test]
    fn a_single_dealer_is_enough_to_start_a_ceremony() {
        let mut deps = init_contract();
        let mut env = mock_env();

        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        // nobody yet, so the window just rolls
        env.block.time = env.block.time.plus_seconds(601);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::PublicKeySubmission { resharing: false },
        )
        .unwrap();

        // then one dealer registers
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 1;
            e
        });

        env.block.time = env.block.time.plus_seconds(601);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::DealingExchange { resharing: false },
        )
        .unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 1);
    }

    /// The same hold applies to resharing, and must not quietly drop the resharing flag.
    #[test]
    fn a_resharing_ceremony_nobody_joined_also_waits() {
        let mut deps = init_contract();
        let mut env = mock_env();

        let epoch = Epoch::new(
            EpochState::PublicKeySubmission { resharing: true },
            7,
            TimeConfiguration::default(),
            env.block.time,
        );
        save_epoch(deps.as_mut().storage, env.block.height, &epoch).unwrap();

        env.block.time = env.block.time.plus_seconds(601);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let current = load_current_epoch(&deps.storage).unwrap();
        assert_eq!(
            current.state,
            EpochState::PublicKeySubmission { resharing: true }
        );
        assert_eq!(current.epoch_id, 7);
    }

    #[test]
    fn verify_threshold() {
        let mut deps = init_contract();
        let mut env = mock_env();
        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        assert!(THRESHOLD.may_load(deps.as_mut().storage).unwrap().is_none());

        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 100;
            e
        });

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
