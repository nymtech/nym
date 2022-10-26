// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;

pub fn compare_decimals(a: Decimal, b: Decimal, epsilon: Option<Decimal>) {
    let epsilon = epsilon.unwrap_or_else(|| Decimal::from_ratio(1u128, 100_000_000u128));
    if a > b {
        assert!(a - b < epsilon, "{} != {}", a, b)
    } else {
        assert!(b - a < epsilon, "{} != {}", a, b)
    }
}
