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
        EpochState::DealingExchange { .. } => {
            let expected_dealings =
                contract_state.key_size * epoch.state_progress.registered_dealers;
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
    use nym_coconut_dkg_common::types::{Epoch, TimeConfiguration};

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
