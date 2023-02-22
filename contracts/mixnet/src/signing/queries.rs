// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::storage::get_signing_nonce;
use cosmwasm_std::{Deps, StdResult};
use nym_contracts_common::signing::Nonce;

pub fn try_get_current_signing_nonce(deps: Deps<'_>, address: String) -> StdResult<Nonce> {
    let address = deps.api.addr_validate(&address)?;
    get_signing_nonce(deps.storage, address)
}
