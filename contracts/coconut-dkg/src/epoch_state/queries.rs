// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{CURRENT_EPOCH, INITIAL_REPLACEMENT_DATA, THRESHOLD};
use crate::error::ContractError;
use coconut_dkg_common::types::{Epoch, InitialReplacementData};
use cosmwasm_std::Storage;

pub fn query_current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    CURRENT_EPOCH
        .load(storage)
        .map_err(|_| ContractError::EpochNotInitialised)
}

pub fn query_current_epoch_threshold(storage: &dyn Storage) -> Result<Option<u64>, ContractError> {
    Ok(THRESHOLD.may_load(storage)?)
}

pub fn query_initial_dealers(
    storage: &dyn Storage,
) -> Result<Option<InitialReplacementData>, ContractError> {
    Ok(INITIAL_REPLACEMENT_DATA.may_load(storage)?)
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::support::tests::helpers::init_contract;
    use coconut_dkg_common::types::{EpochState, TimeConfiguration};
    use cosmwasm_std::testing::mock_env;

    #[test]
    fn query_state() {
        let mut deps = init_contract();
        let epoch = query_current_epoch(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        assert_eq!(
            epoch.finish_timestamp,
            mock_env()
                .block
                .time
                .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs)
        );
    }

    #[test]
    fn query_threshold() {
        let mut deps = init_contract();
        let state = query_current_epoch_threshold(deps.as_mut().storage).unwrap();
        assert!(state.is_none());
    }
}
