// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use nym_geolocation_contract_common::GeolocationContractError;

pub fn try_update_contract_admin(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, GeolocationContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = GEOLOCATION_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?;

    Ok(res)
}
