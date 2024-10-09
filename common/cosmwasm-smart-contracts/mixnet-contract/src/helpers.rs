// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};

#[track_caller]
pub fn compare_decimals(a: Decimal, b: Decimal, epsilon: Option<Decimal>) {
    let epsilon = epsilon.unwrap_or_else(|| Decimal::from_ratio(1u128, 100_000_000u128));
    if a > b {
        assert!(a - b < epsilon, "{a} != {b}")
    } else {
        assert!(b - a < epsilon, "{a} != {b}")
    }
}

pub fn into_base_decimal(val: impl Into<Uint128>) -> StdResult<Decimal> {
    val.into_base_decimal()
}

pub trait IntoBaseDecimal {
    fn into_base_decimal(self) -> StdResult<Decimal>;
}

impl<T> IntoBaseDecimal for T
where
    T: Into<Uint128>,
{
    fn into_base_decimal(self) -> StdResult<Decimal> {
        let atomics = self.into();
        Decimal::from_atomics(atomics, 0).map_err(|_| StdError::GenericErr {
            msg: format!("Decimal range exceeded for {atomics} with 0 decimal places."),
        })
    }
}
