// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ContractError;
use coconut_dkg_common::types::Epoch;
use cosmwasm_std::Storage;
use cw_storage_plus::Item;

pub(crate) const CURRENT_EPOCH: Item<'_, Epoch> = Item::new("current_epoch");

pub(crate) fn current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    CURRENT_EPOCH
        .load(storage)
        .map_err(|_| ContractError::EpochNotInitialised)
}
