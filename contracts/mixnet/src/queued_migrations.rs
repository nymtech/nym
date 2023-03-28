// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage as interval_storage;
use cosmwasm_std::Storage;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::EpochStatus;

pub(crate) fn create_epoch_status(storage: &mut dyn Storage) -> Result<(), MixnetContractError> {
    let current_rewarding_validator =
        crate::mixnet_contract_settings::storage::rewarding_validator_address(storage)?;
    interval_storage::save_current_epoch_status(
        storage,
        &EpochStatus::new(current_rewarding_validator),
    )?;

    Ok(())
}
