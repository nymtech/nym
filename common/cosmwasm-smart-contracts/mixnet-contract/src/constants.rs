// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Decimal, Uint128};

pub const TOKEN_SUPPLY: Uint128 = Uint128::new(1_000_000_000_000_000);

pub const DEFAULT_INTERVAL_OPERATING_COST_AMOUNT: u128 = 40_000_000;
pub const DEFAULT_PROFIT_MARGIN_PERCENT: u64 = 20;

// I'm still not 100% sure how to feel about existence of this file
// This is equivalent of representing our display coin with 6 decimal places.
// I'm using this one as opposed to "Decimal::one()", as this provides us with higher accuracy
// whilst providing no noticable drawbacks.
pub const UNIT_DELEGATION_BASE: Decimal =
    Decimal::raw(1_000_000_000 * 1_000_000_000_000_000_000u128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_delegation_didnt_change() {
        // a sanity check test to make sure Decimal's `DECIMAL_FRACTIONAL` internal implementation hasn't changed
        assert_eq!(
            UNIT_DELEGATION_BASE,
            Decimal::one() * Decimal::from_atomics(1_000_000_000u32, 0).unwrap()
        )
    }
}
