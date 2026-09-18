// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{load_current_epoch, save_epoch, EPOCH_THRESHOLDS, THRESHOLD};
use crate::epoch_state::transactions::reset_dkg_state;
use crate::epoch_state::utils::check_state_completion;
use crate::error::ContractError;
use crate::state::storage::DKG_ADMIN;
use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Response};
use nym_coconut_dkg_common::types::{Epoch, EpochState};

fn ensure_can_advance_state(
    deps: Deps<'_>,
    env: &Env,
    current_epoch: &Epoch,
) -> Result<(), ContractError> {
    if current_epoch.state == EpochState::WaitingInitialisation {
        return Err(ContractError::WaitingInitialisation);
    }

    // an epoch in progress is where the state machine stops. it used to re-save itself here with
    // a fresh deadline, the same id and the same keys, which made `in_progress_time_secs` read
    // like a rotation period while rotating nothing. Rotation is an admin action, via
    // `TriggerReset` / `TriggerResharing`, which do not come through here.
    //
    // this also keeps `ceremony_concluded_at` fixed for as long as the keys are in service,
    // which is what anything sizing a window against their age relies on.
    if current_epoch.state.is_in_progress() {
        return Err(ContractError::EpochAlreadyInProgress);
    }

    // check if we completed the state, so we could short circuit the deadline
    if check_state_completion(deps, current_epoch)? {
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

    advance_epoch_state(deps, env, current_epoch)
}

/// The admin's lever for the phases the contract cannot see the end of.
///
/// Registration completes only against a group that is exactly the participant set, and the
/// multisig vote is invisible from here altogether, so both otherwise burn their whole timer even
/// when the operator can see the work is done. This skips the completion-or-deadline gate and
/// nothing else: the transition is the ordinary one, so the threshold is still fixed on entering
/// the exchange and a sub-threshold finish still resets.
pub(crate) fn try_force_advance_epoch_state(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = load_current_epoch(deps.storage)?;

    // forcing moves a ceremony along; it neither starts one nor rotates keys
    if current_epoch.state == EpochState::WaitingInitialisation {
        return Err(ContractError::WaitingInitialisation);
    }
    if current_epoch.state.is_in_progress() {
        return Err(ContractError::EpochAlreadyInProgress);
    }

    // forcing says "the work is done". Where the contract can already see that it is not -
    // nobody to start with, or too few shares in for the ceremony to ever reach its threshold -
    // going on would only book the automatic reset, so refuse and say why instead
    let progress = current_epoch.state_progress;
    match current_epoch.state {
        EpochState::PublicKeySubmission { .. } if progress.registered_dealers == 0 => {
            return Err(ContractError::NoDealersToAdvance {
                state: current_epoch.state.to_string(),
            });
        }
        EpochState::VerificationKeySubmission { .. } => ensure_threshold_reachable(
            deps.as_ref(),
            &current_epoch,
            progress.submitted_key_shares,
        )?,
        EpochState::VerificationKeyFinalization { .. } => {
            ensure_threshold_reachable(deps.as_ref(), &current_epoch, progress.verified_keys)?
        }
        _ => {}
    }

    let response = advance_epoch_state(deps, env, current_epoch)?;

    // the transition is the ordinary one, so the transaction has to say the phase was cut short
    Ok(response.add_attribute(
        nym_coconut_dkg_common::event_attributes::FORCED_ADVANCE,
        current_epoch.state.to_string(),
    ))
}

/// Refuse a forced advance when `progress` shares can no longer make the epoch's threshold.
fn ensure_threshold_reachable(
    deps: Deps<'_>,
    current_epoch: &Epoch,
    progress: u32,
) -> Result<(), ContractError> {
    let threshold = THRESHOLD.load(deps.storage)?;
    if (progress as u64) < threshold {
        return Err(ContractError::ForcedAdvanceBelowThreshold {
            state: current_epoch.state.to_string(),
            progress,
            threshold,
        });
    }
    Ok(())
}

/// Move the epoch on from `current_epoch`. Whether leaving it is allowed is the caller's question.
fn advance_epoch_state(
    deps: DepsMut<'_>,
    env: Env,
    current_epoch: Epoch,
) -> Result<Response, ContractError> {
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
        // the transaction succeeds either way, so the hold has to announce itself - see
        // the attribute's doc
        return Ok(Response::new().add_attribute(
            nym_coconut_dkg_common::event_attributes::AWAITING_DEALERS,
            extended.epoch_id.to_string(),
        ));
    }

    // `InProgress` is the only state with nothing after it, and both callers have already
    // refused it before getting here
    let Some(next_state) = current_epoch.state.next() else {
        debug_assert!(current_epoch.state.is_in_progress());
        return Err(ContractError::EpochAlreadyInProgress);
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
    use crate::error::ContractError;
    use crate::error::ContractError::EarlyEpochStateAdvancement;
    use crate::state::storage::STATE;
    use crate::support::tests::helpers::{
        group_member, init_contract, init_contract_with_group_members, longer_than_any_phase,
        ADMIN_ADDRESS,
    };
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::{Addr, MessageInfo, Storage};
    use nym_coconut_dkg_common::event_attributes::FORCED_ADVANCE;
    use nym_coconut_dkg_common::types::{StateProgress, TimeConfiguration};

    fn admin() -> MessageInfo {
        message_info(&Addr::unchecked(ADMIN_ADDRESS), &[])
    }

    fn forced_advance_attribute(response: &Response) -> Option<String> {
        response
            .attributes
            .iter()
            .find(|attribute| attribute.key == FORCED_ADVANCE)
            .map(|attribute| attribute.value.clone())
    }

    fn update_epoch<A>(storage: &mut dyn Storage, env: &Env, action: A)
    where
        A: Fn(Epoch) -> Epoch,
    {
        let current = load_current_epoch(storage).unwrap();
        let updated = action(current);
        save_epoch(storage, env.block.height, &updated).unwrap();
    }

    /// B5: advancing an epoch already in progress used to re-save it with a fresh deadline, the
    /// same id and the same keys, so `in_progress_time_secs` read like a rotation period while
    /// nothing rotated. Rotation is an admin action (`TriggerReset` / `TriggerResharing`), which
    /// does not come through here, so this is where the state machine stops.
    ///
    /// It also means the recorded conclusion time stays put, which is what anything sizing a
    /// window against the age of the current keys depends on.
    #[test]
    fn an_epoch_in_progress_is_never_advanced() {
        let mut deps = init_contract();
        let mut env = mock_env();

        update_epoch(deps.as_mut().storage, &env, |epoch| {
            epoch.update(EpochState::InProgress, env.block.time)
        });
        let concluded_at = load_current_epoch(deps.as_ref().storage)
            .unwrap()
            .ceremony_concluded_at;

        // there is no deadline to wait out any more, so let an absurd amount of time pass: this
        // is where the epoch used to quietly extend itself
        assert_eq!(
            load_current_epoch(deps.as_ref().storage).unwrap().deadline,
            None
        );
        env.block.time = env.block.time.plus_seconds(60 * 60 * 24 * 365);

        assert!(matches!(
            try_advance_epoch_state(deps.as_mut(), env.clone()),
            Err(ContractError::EpochAlreadyInProgress)
        ));

        // same epoch, same keys, and the conclusion time was not moved along with it
        let epoch = load_current_epoch(deps.as_ref().storage).unwrap();
        assert_eq!(epoch.state, EpochState::InProgress);
        assert_eq!(epoch.ceremony_concluded_at, concluded_at);
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

        // nor PublicKeySubmission against a group with nobody in it (in either resharing or
        // non-resharing) - `a_full_group_starts_the_ceremony_before_the_deadline` has the rest
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

        // concluding the ceremony records when it happened, and leaves no deadline behind: this
        // is the end of the state machine, not another phase waiting to expire
        assert_eq!(epoch.ceremony_concluded_at, Some(env.block.time));
        assert_eq!(epoch.deadline, None);

        // and it puts this epoch's keys into service, which is what issuance resolves against
        assert_eq!(epoch.keys_in_service, Some(epoch.epoch_id));
        assert_eq!(epoch.issuing_epoch_id(), Some(epoch.epoch_id));

        // so however much time passes, it is never advanced again, and the epoch it settled into
        // keeps both its id and its recorded conclusion
        for skip in [100, 50, 60 * 60 * 24 * 365] {
            env.block.time = env.block.time.plus_seconds(skip);
            assert_eq!(
                try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err(),
                ContractError::EpochAlreadyInProgress
            );
        }

        let unchanged = load_current_epoch(deps.as_mut().storage).unwrap();
        assert_eq!(unchanged, epoch);

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

    /// A ceremony that fails hands the epoch id on and leaves the keys where they were. Anything
    /// deriving the issuing epoch from the id alone would name epoch 11 here: an epoch that
    /// concluded nothing, has no aggregate key, and never will.
    #[test]
    fn a_sub_threshold_ceremony_resets_without_retiring_the_keys_in_service() {
        let mut deps = init_contract();
        let mut env = mock_env();

        // epoch 7's keys are the ones in service, and the ceremony for 11 is about to fail
        THRESHOLD.save(deps.as_mut().storage, &42).unwrap();
        let failing = Epoch {
            state_progress: StateProgress {
                verified_keys: 41,
                ..Default::default()
            },
            keys_in_service: Some(7),
            ..Epoch::new(
                EpochState::VerificationKeyFinalization { resharing: false },
                11,
                TimeConfiguration::default(),
                env.block.time,
            )
        };
        save_epoch(deps.as_mut().storage, env.block.height, &failing).unwrap();

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().verification_key_finalization_time_secs + 1);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let after_failure = load_current_epoch(deps.as_mut().storage).unwrap();
        assert_eq!(12, after_failure.epoch_id);
        assert_eq!(
            EpochState::PublicKeySubmission { resharing: false },
            after_failure.state
        );
        assert_eq!(Some(7), after_failure.issuing_epoch_id());
        assert_eq!(None, after_failure.ceremony_concluded_at);
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
            env.block.time = env.block.time.plus_seconds(longer_than_any_phase());
            let response = try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

            // the hold must say so on the transaction itself: it succeeds, and without the
            // attribute it would be indistinguishable from a real advance without diffing
            // successive epoch queries
            assert!(response.attributes.iter().any(|attribute| {
                attribute.key == nym_coconut_dkg_common::event_attributes::AWAITING_DEALERS
                    && attribute.value == initial_epoch_id.to_string()
            }));

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
        env.block.time = env.block.time.plus_seconds(longer_than_any_phase());
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

        env.block.time = env.block.time.plus_seconds(longer_than_any_phase());
        let response = try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::DealingExchange { resharing: false },
        )
        .unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 1);

        // and a real advance does not claim to be waiting
        assert!(!response.attributes.iter().any(|attribute| {
            attribute.key == nym_coconut_dkg_common::event_attributes::AWAITING_DEALERS
        }));
    }

    /// Registration used to burn its whole timer regardless of who had turned up; with the
    /// group as the definition of "everyone who can", a full house starts the ceremony at once.
    #[test]
    fn a_full_group_starts_the_ceremony_before_the_deadline() {
        let mut deps = init_contract_with_group_members(vec![
            group_member("alice", 1),
            group_member("bob", 1),
            group_member("charlie", 1),
        ]);
        let env = mock_env();

        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        // one still missing: the deadline stands
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 2;
            e
        });
        let err = try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err();
        assert!(matches!(err, EarlyEpochStateAdvancement(_)));

        // everyone in: no waiting, and the threshold is fixed off the full house
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 3;
            e
        });
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::DealingExchange { resharing: false },
        )
        .unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 2);
    }

    #[test]
    fn only_the_admin_can_force_an_advance() {
        let mut deps = init_contract();
        let env = mock_env();
        try_initiate_dkg(deps.as_mut(), env.clone(), admin()).unwrap();
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 2;
            e
        });

        let err = try_force_advance_epoch_state(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked("not the admin"), &[]),
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Admin(_)));
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::PublicKeySubmission { resharing: false },
        )
        .unwrap();
    }

    /// The forced path skips exactly the completion-or-deadline gate: the transition it then
    /// runs is the ordinary one, and the transaction says which phase was cut short.
    #[test]
    fn a_forced_advance_ignores_the_deadline_and_says_so() {
        let mut deps = init_contract();
        let env = mock_env();
        try_initiate_dkg(deps.as_mut(), env.clone(), admin()).unwrap();
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.registered_dealers = 2;
            e
        });

        // the timer still stands for everybody else
        let err = try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap_err();
        assert!(matches!(err, EarlyEpochStateAdvancement(_)));

        let response = try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap();
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::DealingExchange { resharing: false },
        )
        .unwrap();
        assert_eq!(THRESHOLD.load(&deps.storage).unwrap(), 2);
        assert_eq!(
            forced_advance_attribute(&response).as_deref(),
            Some(
                EpochState::PublicKeySubmission { resharing: false }
                    .to_string()
                    .as_str()
            )
        );

        // and a phase that ends on its own merits carries no such mark
        let response = try_advance_epoch_state(
            deps.as_mut(),
            Env {
                block: cosmwasm_std::BlockInfo {
                    time: env.block.time.plus_seconds(longer_than_any_phase()),
                    ..env.block.clone()
                },
                ..env.clone()
            },
        )
        .unwrap();
        assert_eq!(forced_advance_attribute(&response), None);
    }

    /// Forcing means "the work is done", and with nobody registered it plainly is not: the
    /// permissionless hold would extend the window and report success, which is the opposite
    /// of what an operator forcing the phase needs to hear.
    #[test]
    fn a_forced_advance_refuses_to_start_a_ceremony_nobody_joined() {
        let mut deps = init_contract();
        let env = mock_env();
        try_initiate_dkg(deps.as_mut(), env.clone(), admin()).unwrap();
        let before = load_current_epoch(&deps.storage).unwrap();

        let err = try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap_err();
        assert!(matches!(err, ContractError::NoDealersToAdvance { .. }));
        assert_eq!(load_current_epoch(&deps.storage).unwrap(), before);
    }

    /// Where the contract can already see the ceremony would end sub-threshold, forcing it on
    /// would only book the automatic reset - so it is refused, and the operator learns why
    /// instead of losing the whole ceremony.
    #[test]
    fn a_forced_advance_refuses_to_book_a_sub_threshold_ceremony() {
        let mut deps = init_contract();
        let env = mock_env();
        THRESHOLD.save(deps.as_mut().storage, &3).unwrap();

        // too few shares in to ever reach the threshold
        let short_of_shares = Epoch {
            state_progress: StateProgress {
                registered_dealers: 4,
                submitted_key_shares: 2,
                ..Default::default()
            },
            ..Epoch::new(
                EpochState::VerificationKeySubmission { resharing: false },
                11,
                TimeConfiguration::default(),
                env.block.time,
            )
        };
        save_epoch(deps.as_mut().storage, env.block.height, &short_of_shares).unwrap();
        let err = try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap_err();
        assert!(matches!(
            err,
            ContractError::ForcedAdvanceBelowThreshold {
                progress: 2,
                threshold: 3,
                ..
            }
        ));
        assert_eq!(load_current_epoch(&deps.storage).unwrap(), short_of_shares);

        // exactly threshold-many is enough to carry on
        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.submitted_key_shares = 3;
            e
        });
        try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap();
        check_epoch_state(
            deps.as_ref().storage,
            EpochState::VerificationKeyValidation { resharing: false },
        )
        .unwrap();

        // the same at the end: too few verified shares would reset, so it is refused instead
        let short_of_verified = Epoch {
            state_progress: StateProgress {
                registered_dealers: 4,
                submitted_key_shares: 3,
                verified_keys: 2,
                ..Default::default()
            },
            keys_in_service: Some(7),
            ..Epoch::new(
                EpochState::VerificationKeyFinalization { resharing: false },
                11,
                TimeConfiguration::default(),
                env.block.time,
            )
        };
        save_epoch(deps.as_mut().storage, env.block.height, &short_of_verified).unwrap();
        let err = try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap_err();
        assert!(matches!(
            err,
            ContractError::ForcedAdvanceBelowThreshold {
                progress: 2,
                threshold: 3,
                ..
            }
        ));
        // no reset booked: same epoch, same phase, same keys in service
        assert_eq!(
            load_current_epoch(&deps.storage).unwrap(),
            short_of_verified
        );

        update_epoch(deps.as_mut().storage, &env, |mut e| {
            e.state_progress.verified_keys = 3;
            e
        });
        try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap();
        let concluded = load_current_epoch(&deps.storage).unwrap();
        assert_eq!(concluded.state, EpochState::InProgress);
        assert_eq!(concluded.epoch_id, 11);
        assert_eq!(concluded.keys_in_service, Some(11));
        assert_eq!(concluded.outgoing_keys, Some(7));
        assert_eq!(concluded.ceremony_concluded_at, Some(env.block.time));
    }

    /// Forcing moves the ceremony along; it neither starts one nor rotates keys.
    #[test]
    fn a_forced_advance_stops_where_the_state_machine_does() {
        let mut deps = init_contract();
        let env = mock_env();

        let err = try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap_err();
        assert!(matches!(err, ContractError::WaitingInitialisation));

        save_epoch(
            deps.as_mut().storage,
            env.block.height,
            &Epoch::new(
                EpochState::InProgress,
                3,
                TimeConfiguration::default(),
                env.block.time,
            ),
        )
        .unwrap();
        let err = try_force_advance_epoch_state(deps.as_mut(), env.clone(), admin()).unwrap_err();
        assert!(matches!(err, ContractError::EpochAlreadyInProgress));
        assert_eq!(
            load_current_epoch(&deps.storage).unwrap().state,
            EpochState::InProgress
        );
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

        env.block.time = env.block.time.plus_seconds(longer_than_any_phase());
        let response = try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let current = load_current_epoch(&deps.storage).unwrap();
        assert_eq!(
            current.state,
            EpochState::PublicKeySubmission { resharing: true }
        );
        assert_eq!(current.epoch_id, 7);
        assert!(response.attributes.iter().any(|attribute| {
            attribute.key == nym_coconut_dkg_common::event_attributes::AWAITING_DEALERS
                && attribute.value == "7"
        }));
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
