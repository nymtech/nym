// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{load_current_epoch, save_epoch, THRESHOLD};
use crate::error::ContractError;
use crate::state::storage::DKG_ADMIN;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};
use nym_coconut_dkg_common::types::{Epoch, EpochState};

pub use advance_epoch_state::try_advance_epoch_state;

pub mod advance_epoch_state;

fn reset_dkg_state(storage: &mut dyn Storage) -> Result<(), ContractError> {
    THRESHOLD.remove(storage);

    // dealings are preserved in the storage and saved per epoch, so we don't have to do anything about them
    // the same is true for dealer details
    // and epoch progress is reset when new struct is constructed

    Ok(())
}

pub(crate) fn try_initiate_dkg(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to kick start the process
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let epoch = load_current_epoch(deps.storage)?;
    if !matches!(epoch.state, EpochState::WaitingInitialisation) {
        return Err(ContractError::AlreadyInitialised);
    }

    // the first exchange won't involve resharing
    let initial_state = EpochState::PublicKeySubmission { resharing: false };
    let initial_epoch = Epoch::new(initial_state, 0, epoch.time_configuration, env.block.time);
    save_epoch(deps.storage, env.block.height, &initial_epoch)?;

    Ok(Response::default())
}

pub(crate) fn try_trigger_reset(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to trigger DKG reset
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = load_current_epoch(deps.storage)?;

    // only allow reset when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantResetDuringExchange);
    }

    let next_epoch = current_epoch.next_reset(env.block.time);
    save_epoch(deps.storage, env.block.height, &next_epoch)?;

    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

/// The admin's escape hatch: a reset callable from any state past initialisation.
///
/// [`try_trigger_reset`] is deliberately gated on `InProgress`, but a ceremony that keeps ending
/// sub-threshold auto-resets straight into the next attempt without ever getting there, so the
/// ordinary lever can never stop or redirect a looping ceremony. This one can, and it can also
/// abort an exchange already in flight. Aborting leaves the in-flight epoch abandoned, which
/// issuance already tolerates: `keys_in_service` is carried over, not derived from the epoch id.
pub(crate) fn try_trigger_forced_reset(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to force a DKG reset
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = load_current_epoch(deps.storage)?;

    // there is nothing to reset before the DKG has been initiated
    if matches!(current_epoch.state, EpochState::WaitingInitialisation) {
        return Err(ContractError::WaitingInitialisation);
    }

    let next_epoch = current_epoch.next_reset(env.block.time);
    save_epoch(deps.storage, env.block.height, &next_epoch)?;

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
    let current_epoch = load_current_epoch(deps.storage)?;

    // only allow resharing when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = current_epoch.next_resharing(env.block.time);
    save_epoch(deps.storage, env.block.height, &next_epoch)?;

    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::storage::load_current_epoch;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::Addr;
    use cw_controllers::AdminError;

    #[test]
    fn initialising_dkg() {
        let mut deps = init_contract();
        let env = mock_env();

        let initial_epoch_info = load_current_epoch(&deps.storage).unwrap();
        assert!(initial_epoch_info.deadline.is_none());

        let not_admin = deps.api.addr_make("not an admin");
        // can only be executed by the admin
        let res = try_initiate_dkg(deps.as_mut(), env.clone(), message_info(&not_admin, &[]))
            .unwrap_err();
        assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);

        let res = try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        );
        assert!(res.is_ok());

        // can't be initialised more than once
        let res = try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap_err();
        assert_eq!(ContractError::AlreadyInitialised, res);

        // sets the correct epoch data
        let epoch = load_current_epoch(&deps.storage).unwrap();
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

        THRESHOLD.save(deps.as_mut().storage, &42).unwrap();

        reset_dkg_state(deps.as_mut().storage).unwrap();

        assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
    }

    #[cfg(test)]
    mod forced_reset {
        use super::*;
        use nym_coconut_dkg_common::types::{StateProgress, TimeConfiguration};

        #[test]
        fn only_the_admin_may_force_a_reset() {
            let mut deps = init_contract();
            let env = mock_env();

            try_initiate_dkg(
                deps.as_mut(),
                env.clone(),
                message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
            )
            .unwrap();

            let not_admin = deps.api.addr_make("not an admin");
            let res =
                try_trigger_forced_reset(deps.as_mut(), env.clone(), message_info(&not_admin, &[]))
                    .unwrap_err();
            assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);

            // and the epoch was left alone
            assert_eq!(0, load_current_epoch(&deps.storage).unwrap().epoch_id);
        }

        #[test]
        fn there_is_nothing_to_force_before_initialisation() {
            let mut deps = init_contract();
            let env = mock_env();

            let res = try_trigger_forced_reset(
                deps.as_mut(),
                env,
                message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
            )
            .unwrap_err();
            assert_eq!(ContractError::WaitingInitialisation, res);
        }

        /// The reason this message exists: a ceremony that keeps ending sub-threshold auto-resets
        /// into the next attempt without ever reaching `InProgress`, which is the only state the
        /// ordinary `TriggerReset` accepts - so a looping ceremony locks the admin out entirely.
        #[test]
        fn a_forced_reset_escapes_the_sub_threshold_loop() {
            let mut deps = init_contract();
            let mut env = mock_env();
            let admin = message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]);

            // epoch 7's keys are in service and the ceremony for 11 is about to end sub-threshold
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

            env.block.time = env.block.time.plus_seconds(
                TimeConfiguration::default().verification_key_finalization_time_secs + 1,
            );
            try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

            // the loop: a fresh attempt is already running, and the ordinary lever is refused
            // (with the reset error, not the resharing one it used to misreport)
            let looping = load_current_epoch(&deps.storage).unwrap();
            assert_eq!(12, looping.epoch_id);
            assert!(!looping.state.is_in_progress());
            let res = try_trigger_reset(deps.as_mut(), env.clone(), admin.clone()).unwrap_err();
            assert_eq!(ContractError::CantResetDuringExchange, res);

            // the escape hatch is not
            try_trigger_forced_reset(deps.as_mut(), env.clone(), admin).unwrap();

            let after = load_current_epoch(&deps.storage).unwrap();
            assert_eq!(13, after.epoch_id);
            assert_eq!(
                EpochState::PublicKeySubmission { resharing: false },
                after.state
            );
            // the abandoned attempts retired nothing: epoch 7 keeps issuing throughout
            assert_eq!(Some(7), after.issuing_epoch_id());
            assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
        }

        #[test]
        fn a_forced_reset_works_from_every_post_initialisation_state() {
            let states = [
                EpochState::PublicKeySubmission { resharing: false },
                EpochState::PublicKeySubmission { resharing: true },
                EpochState::DealingExchange { resharing: false },
                EpochState::DealingExchange { resharing: true },
                EpochState::VerificationKeySubmission { resharing: false },
                EpochState::VerificationKeySubmission { resharing: true },
                EpochState::VerificationKeyValidation { resharing: false },
                EpochState::VerificationKeyValidation { resharing: true },
                EpochState::VerificationKeyFinalization { resharing: false },
                EpochState::VerificationKeyFinalization { resharing: true },
                EpochState::InProgress,
            ];

            for state in states {
                let mut deps = init_contract();
                let env = mock_env();

                let epoch = Epoch::new(state, 5, TimeConfiguration::default(), env.block.time);
                save_epoch(deps.as_mut().storage, env.block.height, &epoch).unwrap();

                try_trigger_forced_reset(
                    deps.as_mut(),
                    env,
                    message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
                )
                .unwrap();

                let after = load_current_epoch(&deps.storage).unwrap();
                assert_eq!(6, after.epoch_id, "from {state}");
                // always a reset, never a resharing: aborted resharings included, since their
                // registrants may hold nothing to reshare
                assert_eq!(
                    EpochState::PublicKeySubmission { resharing: false },
                    after.state,
                    "from {state}"
                );
            }
        }
    }
}
