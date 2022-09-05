// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::errors::ContractError;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use cosmwasm_std::{DepsMut, Response};
use vesting_contract_common::MigrateMsg;

pub fn migrate_to_v2_mixnet_contract(
    deps: DepsMut<'_>,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &msg.v2_mixnet_contract_address)?;
    Ok(Response::new())
}
