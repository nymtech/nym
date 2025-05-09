// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::f64::consts::LN_2;
use std::time::Duration;

pub(crate) mod background_task;
pub(crate) mod bloomfilter;
pub(crate) mod manager;

pub fn bitmap_size(false_positive_rate: f64, items_in_filter: usize) -> usize {
    /// Equivalent to ln(1 / 2^ln(2)) = −ln^2(2)
    const NEG_LN_2_POW_2: f64 = -0.48045301391820144f64;

    assert!(items_in_filter < f64::MAX.floor() as usize);

    ((items_in_filter as f64 * false_positive_rate.ln()) / NEG_LN_2_POW_2).ceil() as usize
}

#[allow(dead_code)]
pub fn num_of_hash_functions(items_in_filter: usize, bitmap_size: usize) -> usize {
    ((bitmap_size as f64 / items_in_filter as f64) * LN_2).round() as usize
}

pub fn items_in_bloomfilter(reset_rate: Duration, packets_per_second: usize) -> usize {
    reset_rate.as_secs() as usize * packets_per_second
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculating_bitmap_size() {
        let fpr = 1e-5;
        let items_in_filter = 725760000;
        let expected_bitmap_size = 17391129920;

        assert_eq!(bitmap_size(fpr, items_in_filter), expected_bitmap_size);
    }

    #[test]
    fn calculating_number_of_hash_functions() {
        let items_in_filter = 725760000;
        let bitmap_size = 17391129920;
        let expected_hashes = 17;

        assert_eq!(
            num_of_hash_functions(items_in_filter, bitmap_size),
            expected_hashes
        );
    }
}
