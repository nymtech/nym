// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::CURRENT_EPOCH;
use crate::error::ContractError;
use crate::state::storage::STATE;
use cosmwasm_std::Storage;
use nym_coconut_dkg_common::types::{Epoch, EpochState};

// check if we completed the state, so we could short circuit the deadline
pub(crate) fn check_state_completion(
    storage: &dyn Storage,
    epoch: &Epoch,
) -> Result<bool, ContractError> {
    let contract_state = STATE.load(storage)?;

    match epoch.state {
        EpochState::WaitingInitialisation => Ok(false),
        // to check this one we'd need to query for all group members, but we can't rely on this
        EpochState::PublicKeySubmission { .. } => Ok(false),

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

pub(crate) fn check_epoch_state(
    storage: &dyn Storage,
    against: EpochState,
) -> Result<(), ContractError> {
    let epoch_state = CURRENT_EPOCH.load(storage)?.state;
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
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Timestamp;
    use nym_coconut_dkg_common::types::TimeConfiguration;

    #[test]
    fn checking_state_completion() {
        fn epoch_in_state(state: EpochState) -> Epoch {
            Epoch::new(state, 0, Default::default(), Timestamp::from_seconds(69))
        }

        let deps = init_contract();

        // it's never possible to short-circuit `WaitingInitialisation`
        let epoch = epoch_in_state(EpochState::WaitingInitialisation);
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        // neither PublicKeySubmission (in either resharing or non-resharing)
        let epoch = epoch_in_state(EpochState::PublicKeySubmission { resharing: false });
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let epoch = epoch_in_state(EpochState::PublicKeySubmission { resharing: true });
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let key_size = STATE.load(&deps.storage).unwrap().key_size;

        // we can short-circuit `DealingExchange` if all dealers submitted their dealings

        // no dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        // some dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_dealings = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        // all dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_dealings = key_size * 5;
        assert!(check_state_completion(&deps.storage, &epoch).unwrap());

        // no dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        // some dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        epoch.state_progress.submitted_dealings = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        // all dealings
        let mut epoch = epoch_in_state(EpochState::DealingExchange { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.registered_resharing_dealers = 4;
        epoch.state_progress.submitted_dealings = key_size * 4;
        assert!(check_state_completion(&deps.storage, &epoch).unwrap());

        // we can short-circuit `VerificationKeySubmission` if all dealers submitted their verification keys
        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 4;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 4;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: false });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 5;
        assert!(check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeySubmission { resharing: true });
        epoch.state_progress.registered_dealers = 5;
        epoch.state_progress.submitted_key_shares = 5;
        assert!(check_state_completion(&deps.storage, &epoch).unwrap());

        // can't short-circuit `VerificationKeyValidation` => we rely on multisig votes here
        let epoch = epoch_in_state(EpochState::VerificationKeyValidation { resharing: false });
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let epoch = epoch_in_state(EpochState::VerificationKeyValidation { resharing: true });
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        // we can short-circuit `VerificationKeyFinalization` if all submitted keys got verified
        let mut epoch =
            epoch_in_state(EpochState::VerificationKeyFinalization { resharing: false });
        epoch.state_progress.submitted_key_shares = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeyFinalization { resharing: true });
        epoch.state_progress.submitted_key_shares = 5;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch =
            epoch_in_state(EpochState::VerificationKeyFinalization { resharing: false });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 4;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeyFinalization { resharing: true });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 4;
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch =
            epoch_in_state(EpochState::VerificationKeyFinalization { resharing: false });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 5;
        assert!(check_state_completion(&deps.storage, &epoch).unwrap());

        let mut epoch = epoch_in_state(EpochState::VerificationKeyFinalization { resharing: true });
        epoch.state_progress.submitted_key_shares = 5;
        epoch.state_progress.verified_keys = 5;
        assert!(check_state_completion(&deps.storage, &epoch).unwrap());

        // it's never possible to short-circuit `InProgress`
        let epoch = epoch_in_state(EpochState::InProgress);
        assert!(!check_state_completion(&deps.storage, &epoch).unwrap());
    }

    #[test]
    pub fn check_state() {
        let mut deps = init_contract();
        let env = mock_env();

        for fixed_state in EpochState::first().all_until(EpochState::InProgress) {
            CURRENT_EPOCH
                .save(
                    deps.as_mut().storage,
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
