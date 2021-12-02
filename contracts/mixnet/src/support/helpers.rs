// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::gateways::storage as gateways_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{Addr, Storage};

pub fn generate_storage_key(address: &Addr, proxy: Option<&Addr>) -> Vec<u8> {
    if let Some(proxy) = &proxy {
        address
            .as_bytes()
            .iter()
            .zip(proxy.as_bytes())
            .map(|(x, y)| x ^ y)
            .collect()
    } else {
        address.as_bytes().to_vec()
    }
}

// check if the target address has already bonded a mixnode or gateway,
// in either case, return an appropriate error
pub(crate) fn ensure_no_existing_bond(
    storage: &dyn Storage,
    sender: &Addr,
) -> Result<(), ContractError> {
    if mixnodes_storage::mixnodes()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsMixnode);
    }

    if gateways_storage::gateways()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsGateway);
    }

    Ok(())
}
