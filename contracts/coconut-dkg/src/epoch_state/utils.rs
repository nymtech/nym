// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{BLOCK_TIME_FOR_VERIFICATION_SECS, MAX_PHASE_DURATION_SECS};
use crate::epoch_state::storage::load_current_epoch;
use crate::error::ContractError;
use crate::state::storage::STATE;
use cosmwasm_std::{Deps, StdResult, Storage};
use cw4::Cw4Contract;
use nym_coconut_dkg_common::types::{Epoch, EpochState, TimeConfiguration};

// check if we completed the state, so we could short circuit the deadline
pub(crate) fn check_state_completion(deps: Deps<'_>, epoch: &Epoch) -> Result<bool, ContractError> {
    let contract_state = STATE.load(deps.storage)?;

    match epoch.state {
        EpochState::WaitingInitialisation => Ok(false),

        // only a voting member of the group can register (`ensure_group_member`), so once every
        // one of them has there is nobody left to wait for. A member that never turns up leaves
        // this false and the phase to its deadline, which is the direction the mistake is allowed
        // in: nobody who could still join is ever shut out early
        EpochState::PublicKeySubmission { .. } => {
            // a group the contract cannot read is one it can conclude nothing from, so fall
            // through to the deadline rather than fail the advance outright
            let Ok(voting_members) = count_voting_members(deps, &contract_state.group_addr) else {
                return Ok(false);
            };

            // `>=` rather than `==`: a member removed after registering must not leave the
            // phase unable to conclude. An empty group has nobody to register, and the
            // zero-dealer hold in `try_advance_epoch_state` owns that case
            Ok(voting_members > 0 && epoch.state_progress.registered_dealers >= voting_members)
        }

        // if every dealer has submitted all dealings, we're done
        EpochState::DealingExchange { resharing } => {
            // during resharing, we only expect to receive dealings from resharing dealers
            let expected_dealings = if !resharing {
                contract_state.key_size * epoch.state_progress.registered_dealers
            } else {
                contract_state.key_size * epoch.state_progress.registered_resharing_dealers
            };

            Ok(expected_dealings == epoch.state_progress.submitted_dealings)
        }

        // if every dealer has submitted its partial key, we're done
        EpochState::VerificationKeySubmission { .. } => Ok(epoch
            .state_progress
            .submitted_key_shares
            == epoch.state_progress.registered_dealers),

        // no short-circuiting this one since the voting is happening in the multisig contract
        EpochState::VerificationKeyValidation { .. } => Ok(false),

        // if every submitted partial key has been verified, we're done
        EpochState::VerificationKeyFinalization { .. } => {
            Ok(epoch.state_progress.verified_keys == epoch.state_progress.submitted_key_shares)
        }
        EpochState::InProgress => Ok(false),
    }
}

/// How many members of the group could register, i.e. carry a voting weight.
///
/// cw4-group caps a `ListMembers` page at 30, so the group is paged through in address order.
fn count_voting_members(deps: Deps<'_>, group: &Cw4Contract) -> StdResult<u32> {
    const PAGE_SIZE: u32 = 30;

    let mut voting_members = 0;
    let mut start_after = None;
    loop {
        let page = group.list_members(&deps.querier, start_after, Some(PAGE_SIZE))?;
        voting_members += page.iter().filter(|member| member.weight >= 1).count() as u32;

        if page.len() < PAGE_SIZE as usize {
            return Ok(voting_members);
        }
        start_after = page.last().map(|member| member.addr.clone());
    }
}

/// Refuse timings the ceremony could not run under.
///
/// Every phase needs a duration, or it is advanceable the moment it is entered, and none may
/// exceed `MAX_PHASE_DURATION_SECS`, past which the deadline arithmetic would overflow. And a share
/// committed at the very start of `VerificationKeySubmission` gets `BLOCK_TIME_FOR_VERIFICATION_SECS`
/// to be voted on and executed, which has to outlast the rest of submission, all of validation
/// and all of finalization - otherwise the proposal expires before finalization can execute it,
/// the share silently never verifies, and enough of those reset the ceremony.
pub(crate) fn ensure_valid_time_configuration(
    config: &TimeConfiguration,
) -> Result<(), ContractError> {
    let phases = [
        (
            "public key submission",
            config.public_key_submission_time_secs,
        ),
        ("dealing exchange", config.dealing_exchange_time_secs),
        (
            "verification key submission",
            config.verification_key_submission_time_secs,
        ),
        (
            "verification key validation",
            config.verification_key_validation_time_secs,
        ),
        (
            "verification key finalization",
            config.verification_key_finalization_time_secs,
        ),
    ];
    if let Some((phase, _)) = phases.iter().find(|(_, duration)| *duration == 0) {
        return Err(ContractError::ZeroPhaseDuration { phase });
    }
    if let Some((phase, duration)) = phases
        .iter()
        .find(|(_, duration)| *duration > MAX_PHASE_DURATION_SECS)
    {
        return Err(ContractError::PhaseDurationTooLong {
            phase,
            duration: *duration,
            limit: MAX_PHASE_DURATION_SECS,
        });
    }

    let total = config.verification_key_submission_time_secs
        + config.verification_key_validation_time_secs
        + config.verification_key_finalization_time_secs;
    if total >= BLOCK_TIME_FOR_VERIFICATION_SECS {
        return Err(ContractError::VerificationPhasesOutlastProposals {
            total,
            limit: BLOCK_TIME_FOR_VERIFICATION_SECS,
        });
    }

    Ok(())
}

pub(crate) fn check_epoch_state(
    storage: &dyn Storage,
    against: EpochState,
) -> Result<(), ContractError> {
    let epoch_state = load_current_epoch(storage)?.state;
    if epoch_state != against {
        Err(ContractError::IncorrectEpochState {
            current_state: epoch_state.to_string(),
            expected_state: against.to_string(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::epoch_state::storage::save_epoch;
    use crate::support::tests::helpers::{
        group_member, init_contract, init_contract_with_group_members, GROUP_CONTRACT,
    };
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{SystemError, SystemResult, Timestamp};
    use nym_coconut_dkg_common::types::TimeConfiguration;

    #[test]
    fn checking_state_completion() {
        fn epoch_in_state(state: EpochState) -> Epoch {
            Epoch::new(state, 0, Default::default(), Timestamp::from_seconds(69))
        }

        let deps = init_contract();

        // it's never possible to short-circuit `WaitingInitialisation`
        let epoch = epoch_in_state(EpochState::WaitingInitialisation);
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        // nor PublicKeySubmission against a group with nobody in it (in either resharing or
        // non-resharing) - the group-driven cases have their own tests below
        let epoch = epoch_in_state(EpochState::PublicKeySubmission { resharing: false });
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let epoch = epoch_in_state(EpochState::PublicKeySubmission { resharing: true });
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let key_size = STATE.load(&deps.storage).unwrap().key_size;

        // we can short-circuit `DealingExchange` if all dealers submitted their dealings

        // no dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        // some dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_dealings = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        // all dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_dealings = key_size * 5;
        assert!(check_state_completion(deps.as_ref(), &epoch).unwrap());

        // no dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        // some dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        epoch.state_progress.submitted_dealings = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        // all dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        epoch.state_progress.submitted_dealings = key_size * 4;
        assert!(check_state_completion(deps.as_ref(), &epoch).unwrap());

        // we can short-circuit `VerificationKeySubmission` if all dealers submitted their verification keys
        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 4;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 4;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 5;
        assert!(check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 5;
        assert!(check_state_completion(deps.as_ref(), &epoch).unwrap());

        // can't short-circuit `VerificationKeyValidation` => we rely on multisig votes here
        let epoch = epoch_in_state(EpochState::VerificationKeyValidation { resharing: false });
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let epoch = epoch_in_state(EpochState::VerificationKeyValidation { resharing: true });
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        // we can short-circuit `VerificationKeyFinalization` if all submitted keys got verified
        let mut epoch =
            epoch_in_state(EpochState::VerificationKeyFinalization { resharing: false });
        epoch.state_progress.submitted_key_shares = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeyFinalization { resharing: true });
        epoch.state_progress.submitted_key_shares = 5;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch =
            epoch_in_state(EpochState::VerificationKeyFinalization { resharing: false });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 4;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeyFinalization { resharing: true });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 4;
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch =
            epoch_in_state(EpochState::VerificationKeyFinalization { resharing: false });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 5;
        assert!(check_state_completion(deps.as_ref(), &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeyFinalization { resharing: true });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 5;
        assert!(check_state_completion(deps.as_ref(), &epoch).unwrap());

        // it's never possible to short-circuit `InProgress`
        let epoch = epoch_in_state(EpochState::InProgress);
        assert!(!check_state_completion(deps.as_ref(), &epoch).unwrap());
    }

    fn registration_with(registered_dealers: u32) -> Epoch {
        let mut epoch = Epoch::new(
            EpochState::PublicKeySubmission { resharing: false },
            0,
            Default::default(),
            Timestamp::from_seconds(69),
        );
        epoch.state_progress.registered_dealers = registered_dealers;
        epoch
    }

    /// Only voting members of the group can register (`ensure_group_member`), so once every
    /// one of them has, there is nobody left for the phase to wait for.
    #[test]
    fn registration_concludes_once_every_voting_member_is_in() {
        let deps = init_contract_with_group_members(vec![
            group_member("alice", 1),
            group_member("bob", 1),
            group_member("charlie", 1),
            // a zero-weight member cannot register, so it is not somebody to wait for
            group_member("dave", 0),
        ]);

        assert!(!check_state_completion(deps.as_ref(), &registration_with(0)).unwrap());
        assert!(!check_state_completion(deps.as_ref(), &registration_with(2)).unwrap());
        assert!(check_state_completion(deps.as_ref(), &registration_with(3)).unwrap());

        // a member removed from the group after registering must not leave the phase
        // unable to conclude
        assert!(check_state_completion(deps.as_ref(), &registration_with(4)).unwrap());

        // the resharing flag changes nothing about who is expected
        let mut resharing = registration_with(3);
        resharing.state = EpochState::PublicKeySubmission { resharing: true };
        assert!(check_state_completion(deps.as_ref(), &resharing).unwrap());
    }

    /// A group the contract cannot read is one it can conclude nothing from: the phase falls
    /// through to its deadline rather than the advance failing outright.
    #[test]
    fn an_unreadable_group_leaves_registration_to_its_deadline() {
        let mut deps = init_contract();
        deps.querier.update_wasm(|_| {
            SystemResult::Err(SystemError::NoSuchContract {
                addr: GROUP_CONTRACT.to_string(),
            })
        });

        assert!(!check_state_completion(deps.as_ref(), &registration_with(3)).unwrap());
    }

    #[test]
    fn verification_phases_must_fit_inside_a_proposal_lifetime() {
        let limit = BLOCK_TIME_FOR_VERIFICATION_SECS;
        let with_verification_phases = |submission, validation, finalization| TimeConfiguration {
            verification_key_submission_time_secs: submission,
            verification_key_validation_time_secs: validation,
            verification_key_finalization_time_secs: finalization,
            ..TimeConfiguration::default()
        };

        assert!(ensure_valid_time_configuration(&TimeConfiguration::default()).is_ok());

        // exactly the limit is one second too many; a second under fits
        let at_limit = with_verification_phases(limit / 2, limit / 4, limit / 4);
        assert!(matches!(
            ensure_valid_time_configuration(&at_limit),
            Err(ContractError::VerificationPhasesOutlastProposals { total, .. }) if total == limit
        ));
        let under_limit = with_verification_phases(limit / 2, limit / 4, limit / 4 - 1);
        assert!(ensure_valid_time_configuration(&under_limit).is_ok());

        // and no phase may have no duration
        let no_validation = with_verification_phases(600, 0, 600);
        assert!(matches!(
            ensure_valid_time_configuration(&no_validation),
            Err(ContractError::ZeroPhaseDuration {
                phase: "verification key validation"
            })
        ));
    }

    /// The cap is on the phase, not the sum: each one may sit exactly at it, none may pass it.
    #[test]
    fn a_phase_cannot_outlast_the_cap() {
        let with_registration = |duration| TimeConfiguration {
            public_key_submission_time_secs: duration,
            ..TimeConfiguration::default()
        };

        assert!(
            ensure_valid_time_configuration(&with_registration(MAX_PHASE_DURATION_SECS)).is_ok()
        );
        assert!(matches!(
            ensure_valid_time_configuration(&with_registration(MAX_PHASE_DURATION_SECS + 1)),
            Err(ContractError::PhaseDurationTooLong {
                phase: "public key submission",
                duration,
                limit: MAX_PHASE_DURATION_SECS,
            }) if duration == MAX_PHASE_DURATION_SECS + 1
        ));

        // the case the cap exists for: a value that would overflow the deadline arithmetic
        assert!(matches!(
            ensure_valid_time_configuration(&with_registration(u64::MAX)),
            Err(ContractError::PhaseDurationTooLong { .. })
        ));
    }

    /// cw4-group hands out at most 30 members per page, so a bigger group has to be paged
    /// through rather than counted from its first page.
    #[test]
    fn a_group_larger_than_one_page_is_counted_in_full() {
        let members = (0..35)
            .map(|i| group_member(&format!("member{i:02}"), 1))
            .collect();
        let deps = init_contract_with_group_members(members);

        assert!(!check_state_completion(deps.as_ref(), &registration_with(30)).unwrap());
        assert!(!check_state_completion(deps.as_ref(), &registration_with(34)).unwrap());
        assert!(check_state_completion(deps.as_ref(), &registration_with(35)).unwrap());
    }

    #[test]
    pub fn check_state() {
        let mut deps = init_contract();
        let env = mock_env();

        for fixed_state in EpochState::first().all_until(EpochState::InProgress) {
            save_epoch(
                deps.as_mut().storage,
                env.block.height,
                &Epoch::new(fixed_state, 0, TimeConfiguration::default(), env.block.time),
            )
            .unwrap();
            for against_state in EpochState::first().all_until(EpochState::InProgress) {
                let ret = check_epoch_state(deps.as_mut().storage, against_state);
                if fixed_state == against_state {
                    assert!(ret.is_ok());
                } else {
                    assert!(ret.is_err());
                }
            }
        }
    }
}
