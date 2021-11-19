// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{circulating_supply, config_read, read_layer_distribution, reward_pool_value};

use cosmwasm_std::{Deps, Uint128};
use mixnet_contract::{LayerDistribution, RewardingIntervalResponse};

pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 100;
pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

// currently the maximum limit before running into memory issue is somewhere between 1150 and 1200
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 750;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 500;

pub(crate) fn query_rewarding_interval(deps: Deps) -> RewardingIntervalResponse {
    let state = config_read(deps.storage).load().unwrap();
    RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    }
}

pub(crate) fn query_reward_pool(deps: Deps) -> Uint128 {
    reward_pool_value(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps) -> Uint128 {
    circulating_supply(deps.storage)
}

/// Adds a 0 byte to terminate the `start_after` value given. This allows CosmWasm
/// to get the succeeding key as the start of the next page.
// S works for both `String` and `Addr` and that's what we wanted
pub fn calculate_start_value<S: AsRef<str>>(start_after: Option<S>) -> Option<Vec<u8>> {
    start_after.as_ref().map(|identity| {
        identity
            .as_ref()
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect()
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::error::ContractError;
    use crate::mixnodes::delegation_queries::query_mixnode_delegation;
    use crate::storage::mix_delegations;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::raw_delegation_fixture;
    use config::defaults::DENOM;
    use cosmwasm_std::coin;
    use cosmwasm_std::{Addr, Storage};
    use mixnet_contract::Delegation;
    use mixnet_contract::IdentityKey;
    use mixnet_contract::RawDelegationData;
}
