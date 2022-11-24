// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::CURRENT_EPOCH_STATE;
use crate::error::ContractError;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::Storage;

pub(crate) fn query_current_epoch_state(
    storage: &dyn Storage,
) -> Result<EpochState, ContractError> {
    CURRENT_EPOCH_STATE
        .load(storage)
        .map_err(|_| ContractError::EpochNotInitialised)
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::support::tests::helpers::init_contract;

    #[test]
    fn query_state() {
        let mut deps = init_contract();
        let state = query_current_epoch_state(deps.as_mut().storage).unwrap();
        assert_eq!(state, EpochState::PublicKeySubmission);
    }
}
