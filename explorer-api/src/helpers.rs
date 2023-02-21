// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnet_contract_common::{Decimal, Fraction};

pub(crate) fn best_effort_small_dec_to_f64(dec: Decimal) -> f64 {
    let num = dec.numerator().u128() as f64;
    let den = dec.denominator().u128() as f64;
    num / den
}
