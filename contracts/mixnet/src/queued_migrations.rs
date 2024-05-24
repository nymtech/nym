// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::DepsMut;
use mixnet_contract_common::error::MixnetContractError;

pub(crate) fn vesting_purge(deps: DepsMut) -> Result<(), MixnetContractError> {
    todo!("ensure no pending")
}
