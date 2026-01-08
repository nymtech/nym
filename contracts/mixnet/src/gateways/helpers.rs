// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use cosmwasm_std::{Addr, Storage};
use nym_mixnet_contract_common::{GatewayBond, error::MixnetContractError};

pub(crate) fn must_get_gateway_bond_by_owner(
    store: &dyn Storage,
    owner: &Addr,
) -> Result<GatewayBond, MixnetContractError> {
    Ok(storage::gateways()
        .idx
        .owner
        .item(store, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedGatewayBond {
            owner: owner.clone(),
        })?
        .1)
}
