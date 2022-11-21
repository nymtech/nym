// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ContractError, CURRENT_EPOCH_STATE};
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::Storage;

pub(crate) fn check_epoch_state(
    storage: &dyn Storage,
    against: EpochState,
) -> Result<(), ContractError> {
    let epoch_state = CURRENT_EPOCH_STATE.load(storage)?;
    if epoch_state != against {
        Err(ContractError::IncorrectEpochState {
            current_state: epoch_state.to_string(),
            expected_state: against.to_string(),
        })
    } else {
        Ok(())
    }
}
