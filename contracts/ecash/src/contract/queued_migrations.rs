// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::deposit::DepositStorage;
use crate::deposit_stats::DepositStatsStorage;
use cosmwasm_std::DepsMut;
use cw_storage_plus::Item;
use nym_ecash_contract_common::counters::PoolCounters;
use nym_ecash_contract_common::EcashContractError;

pub fn add_tiered_pricing(deps: DepsMut) -> Result<(), EcashContractError> {
    let deposits = DepositStorage::new();
    let deposits_stats = DepositStatsStorage::new();
    let pool_counters: Item<PoolCounters> = Item::new("pool_counters");

    // all the deposits made so far were performed with the default price

    // initialise the storage Item with the current number of deposits
    let deposits_performed = match deposits.deposit_id_counter.may_load(deps.storage)? {
        Some(id) => {
            // note: the first deposit had id of 0, so we had to increment it by 1
            id + 1
        }
        None => 0,
    };

    let deposits_amounts = pool_counters.load(deps.storage)?.total_deposited;

    deposits_stats
        .deposits_with_default_price
        .save(deps.storage, &deposits_performed)?;

    deposits_stats
        .deposits_with_default_price_amounts
        .save(deps.storage, &deposits_amounts)?;

    Ok(())
}
