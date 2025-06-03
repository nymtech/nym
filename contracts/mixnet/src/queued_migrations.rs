// Copyright 2022-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage as interval_storage;
use crate::nodes::storage as nymnodes_storage;
use cosmwasm_std::DepsMut;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::KeyRotationState;

pub fn introduce_key_rotation_id(deps: DepsMut) -> Result<(), MixnetContractError> {
    let current_epoch_id =
        interval_storage::current_interval(deps.storage)?.current_epoch_absolute_id();
    nymnodes_storage::KEY_ROTATION_STATE.save(
        deps.storage,
        &KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: current_epoch_id,
        },
    )?;
    Ok(())
}
