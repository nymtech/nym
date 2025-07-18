// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::HISTORICAL_EPOCH;
use crate::error::ContractError;
use cosmwasm_std::{DepsMut, Env};

pub fn introduce_historical_epochs(deps: DepsMut, env: Env) -> Result<(), ContractError> {
    if HISTORICAL_EPOCH.may_load(deps.storage)?.is_some() {
        return Err(ContractError::FailedMigration {
            comment: "this migration has already been run before".to_string(),
        });
    }

    #[allow(deprecated)]
    let current = crate::epoch_state::storage::CURRENT_EPOCH.load(deps.storage)?;
    // we won't have information on intermediate states prior to now, but that's not the end of the world
    HISTORICAL_EPOCH.save(deps.storage, &current, env.block.height)?;

    Ok(())
}
