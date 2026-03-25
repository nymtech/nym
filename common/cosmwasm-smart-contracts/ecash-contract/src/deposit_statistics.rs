// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use std::collections::HashMap;

/// Aggregate statistics about all deposits made through the ecash contract.
#[cw_serde]
pub struct DepositsStatistics {
    /// Total number of deposits ever made (at any price tier),
    /// derived from the deposit id counter.
    pub total_deposits_made: u32,

    /// Total value of all deposits ever made (at any price tier),
    /// sourced from `PoolCounters::total_deposited`.
    pub total_deposited: Coin,

    /// Number of deposits made at the default (non-reduced) price.
    pub total_deposits_made_with_default_price: u32,

    /// Total value deposited at the default price.
    pub total_deposited_with_default_price: Coin,

    /// Number of deposits made at any custom (reduced) price, summed across all whitelisted accounts.
    pub total_deposits_made_with_custom_price: u32,

    /// Total value deposited at custom prices, summed across all whitelisted accounts.
    pub total_deposited_with_custom_price: Coin,

    /// Per-account breakdown of deposit counts for whitelisted addresses.
    // note: we use String for addressing due to serialisation incompatibility
    pub deposits_made_with_custom_price: HashMap<String, u32>,

    /// Per-account breakdown of deposited amounts for whitelisted addresses.
    // note: we use String for addressing due to serialisation incompatibility
    pub deposited_with_custom_price: HashMap<String, Coin>,
}
