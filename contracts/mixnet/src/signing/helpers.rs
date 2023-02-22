// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::storage;
use cosmwasm_std::{Addr, Storage};
use mixnet_contract_common::error::MixnetContractError;

fn construct_signed_message<T>(
    storage: &mut dyn Storage,
    signer: Addr,
    message: T,
) -> Result<(), MixnetContractError> {
    // right now all signing is done with ed25519 so that simplifies things a bit
    let nonce = storage::get_and_update_signing_nonce(storage, signer)?;

    todo!()
}
