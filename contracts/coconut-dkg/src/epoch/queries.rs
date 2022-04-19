// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::ContractError;
use coconut_dkg_common::types::Epoch;
use cosmwasm_std::Storage;

pub(crate) fn query_current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    storage::current_epoch(storage)
}
