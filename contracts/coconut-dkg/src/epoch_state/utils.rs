// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::CURRENT_EPOCH;
use crate::error::ContractError;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::Storage;

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
    use coconut_dkg_common::types::{Epoch, TimeConfiguration};
    use cosmwasm_std::testing::mock_env;

    #[test]
    pub fn check_state() {
        let mut deps = init_contract();
        let env = mock_env();

        for fixed_state in EpochState::default().all_until(EpochState::InProgress) {
            CURRENT_EPOCH
                .save(
                    deps.as_mut().storage,
                    &Epoch::new(fixed_state, 0, TimeConfiguration::default(), env.block.time),
                )
                .unwrap();
            for against_state in EpochState::default().all_until(EpochState::InProgress) {
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
