// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::Storage;

pub(crate) fn query_current_epoch_state(
    storage: &dyn Storage,
) -> Result<EpochState, ContractError> {
    storage::current_epoch_state(storage)
}
