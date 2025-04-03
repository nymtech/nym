// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config;
use std::f64::consts::LN_2;

pub(crate) mod background_task;
pub(crate) mod bloomfilter;

pub struct Config {
    /// Probability of false positives, fraction between 0 and 1 or a number indicating 1-in-p
    pub false_positive_rate: f64,

    pub items_in_filter: usize,

    pub bitmap_size: usize,

    pub hash_functions: usize,
}

pub fn bitmap_size(false_positive_rate: f64, items_in_filter: usize) -> usize {
    /// Equivalent to ln(1 / 2^ln(2)) = −ln^2(2)
    const NEG_LN_2_POW_2: f64 = -0.480453013918201424667102526326664972_f64;

    assert!(items_in_filter < f64::MAX.floor() as usize);
    // TODO: should this be div by 8?

    ((items_in_filter as f64 * false_positive_rate.ln()) / NEG_LN_2_POW_2).ceil() as usize
}

pub fn num_of_hash_functions(items_in_filter: usize, bitmap_size: usize) -> usize {
    ((bitmap_size as f64 / items_in_filter as f64) * LN_2).round() as usize
}

impl From<config::ReplayProtection> for Config {
    fn from(value: config::ReplayProtection) -> Self {
        todo!()
        // let items_in_filter = value.bloomfilter_clear_rate.as_secs() as usize
        //     * value.initial_expected_packets_per_second;
        // let bitmap_size = bitmap_size(value.false_positive_rate, items_in_filter);
        // let hash_functions = num_of_hash_functions(items_in_filter, bitmap_size);
        //
        // Config {
        //     false_positive_rate: value.false_positive_rate,
        //     items_in_filter,
        //     bitmap_size,
        //     hash_functions,
        // }
    }
}

// impl From<Config> for BloomfilterParameters {}

impl Config {
    pub const fn byte_size(&self) -> usize {
        self.bitmap_size / 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::bloomfilter::Bloom;

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
