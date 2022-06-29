// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;

// I'm still not 100% sure how to feel about existence of this file
// This is equivalent of representing our display coin with 6 decimal places.
// I'm using this one as opposed to "Decimal::one()", as this provides us with higher accuracy
// whilst providing no noticable drawbacks.
pub const UNIT_DELEGATION_BASE: Decimal = Decimal::raw(1000000 * 1_000_000_000_000_000_000u128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_delegation_didnt_change() {
        // a sanity check test to make sure Decimal's `DECIMAL_FRACTIONAL` internal implementation hasn't changed
        assert_eq!(
            UNIT_DELEGATION_BASE,
            Decimal::one() * Decimal::from_atomics(1000000u32, 0).unwrap()
        )
    }
}
