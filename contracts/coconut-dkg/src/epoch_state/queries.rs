// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{CURRENT_EPOCH, INITIAL_REPLACEMENT_DATA, THRESHOLD};
use crate::error::ContractError;
use cosmwasm_std::Storage;
use nym_coconut_dkg_common::types::{Epoch, InitialReplacementData};

pub(crate) fn query_current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    CURRENT_EPOCH
        .load(storage)
        .map_err(|_| ContractError::EpochNotInitialised)
}

pub(crate) fn query_current_epoch_threshold(
    storage: &dyn Storage,
) -> Result<Option<u64>, ContractError> {
    Ok(THRESHOLD.may_load(storage)?)
}

pub(crate) fn query_initial_dealers(
    storage: &dyn Storage,
) -> Result<Option<InitialReplacementData>, ContractError> {
    Ok(INITIAL_REPLACEMENT_DATA.may_load(storage)?)
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::epoch_state::transactions::try_initiate_dkg;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use nym_coconut_dkg_common::types::{EpochState, TimeConfiguration};

    #[test]
    fn query_state() {
        let mut deps = init_contract();
        let epoch = query_current_epoch(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::WaitingInitialisation);
        assert_eq!(epoch.deadline, None);

        let env = mock_env();
        try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let epoch = query_current_epoch(deps.as_mut().storage).unwrap();
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        assert_eq!(
            epoch.deadline.unwrap(),
            env.block
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
