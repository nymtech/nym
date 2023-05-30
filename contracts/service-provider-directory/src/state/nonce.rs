// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::SIGNING_NONCES_NAMESPACE, error::Result};

use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::Map;
use nym_contracts_common::signing::Nonce;

pub const NONCES: Map<'_, Addr, Nonce> = Map::new(SIGNING_NONCES_NAMESPACE);

pub fn get_signing_nonce(storage: &dyn Storage, address: Addr) -> Result<Nonce> {
    let nonce = NONCES.may_load(storage, address)?.unwrap_or(0);
    Ok(nonce)
}

pub fn update_signing_nonce(storage: &mut dyn Storage, address: Addr, value: Nonce) -> Result<()> {
    NONCES
        .save(storage, address, &value)
        .map_err(|err| err.into())
}

pub fn increment_signing_nonce(storage: &mut dyn Storage, address: Addr) -> Result<()> {
    // get the current nonce
    let nonce = get_signing_nonce(storage, address.clone())?;

    // increment it for the next use
    update_signing_nonce(storage, address, nonce + 1)
}
