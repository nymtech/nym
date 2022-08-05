// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{DepsMut, Env, Response};
use vesting_contract_common::MigrateMsg;

use crate::{errors::ContractError, storage::MIX_DENOM};

pub fn migrate_config_from_env(
    deps: DepsMut<'_>,
    _env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    MIX_DENOM.save(deps.storage, &msg.mix_denom)?;

    Ok(Default::default())
}
