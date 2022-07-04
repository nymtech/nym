// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use cosmwasm_std::{Addr, StdResult, Storage};
use mixnet_contract_common::mixnode::MixNodeDetails;

pub(crate) fn get_mixnode_details_by_owner(
    store: &dyn Storage,
    address: Addr,
) -> StdResult<Option<MixNodeDetails>> {
    if let Some(bond_information) = storage::mixnode_bonds()
        .idx
        .owner
        .item(store, address)?
        .map(|record| record.1)
    {
        // if bond exists, rewarding details MUST also exist
        let rewarding_details = storage::MIXNODE_REWARDING.load(store, bond_information.id)?;
        Ok(Some(MixNodeDetails::new(
            bond_information,
            rewarding_details,
        )))
    } else {
        Ok(None)
    }
}
