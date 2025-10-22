// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Scalar (non-SIMD) implementation of bitmap operations.
//! Used as a fallback when SIMD instructions are unavailable.

use super::BitmapOps;

/// Scalar (non-SIMD) bitmap operations implementation
pub struct ScalarBitmapOps;

impl BitmapOps for ScalarBitmapOps {
    #[inline(always)]
    fn clear_words(bitmap: &mut [u64], start_idx: usize, num_words: usize) {
        for i in start_idx..(start_idx + num_words) {
            bitmap[i] = 0;
        }
    }

    #[inline(always)]
    fn is_range_zero(bitmap: &[u64], start_idx: usize, num_words: usize) -> bool {
        for i in start_idx..(start_idx + num_words) {
            if bitmap[i] != 0 {
                return false;
            }
        }
        true
    }

    #[inline(always)]
    fn set_bit(bitmap: &mut [u64], bit_idx: u64) {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = (bit_idx % 64) as u64;
        bitmap[word_idx] |= 1u64 << bit_pos;
    }

    #[inline(always)]
    fn clear_bit(bitmap: &mut [u64], bit_idx: u64) {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = (bit_idx % 64) as u64;
        bitmap[word_idx] &= !(1u64 << bit_pos);
    }

    #[inline(always)]
    fn check_bit(bitmap: &[u64], bit_idx: u64) -> bool {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = (bit_idx % 64) as u64;
        (bitmap[word_idx] & (1u64 << bit_pos)) != 0
    }
}

/// Scalar implementations of other bitmap utilities
pub mod atomic {
    /// Check and set bit, returning the previous state
    /// This function is not actually atomic! It's just a normal operation
    #[inline(always)]
    pub fn check_and_set_bit(bitmap: &mut [u64], bit_idx: u64) -> bool {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = (bit_idx % 64) as u64;
        let mask = 1u64 << bit_pos;

        // Get old value
        let old_word = bitmap[word_idx];

        // Set bit regardless of current state
        bitmap[word_idx] |= mask;

        // Return true if bit was already set (duplicate)
        (old_word & mask) != 0
    }

    /// Set a range of bits efficiently
    #[inline(always)]
    pub fn set_bits_range(bitmap: &mut [u64], start_bit: u64, end_bit: u64) {
        // Process whole words where possible
        let start_word = (start_bit / 64) as usize;
        let end_word = (end_bit / 64) as usize;

        if start_word == end_word {
            // Special case: all bits in the same word
            let start_mask = u64::MAX << (start_bit % 64);
            let end_mask = u64::MAX >> (63 - (end_bit % 64));
            bitmap[start_word] |= start_mask & end_mask;
            return;
        }

        // Handle partial words at the beginning and end
        if start_bit % 64 != 0 {
            let start_mask = u64::MAX << (start_bit % 64);
            bitmap[start_word] |= start_mask;
        }

        if (end_bit + 1) % 64 != 0 {
            let end_mask = u64::MAX >> (63 - (end_bit % 64));
            bitmap[end_word] |= end_mask;
        }

        // Handle complete words in the middle
        let first_full_word = if start_bit % 64 == 0 {
            start_word
        } else {
            start_word + 1
        };
        let last_full_word = if (end_bit + 1) % 64 == 0 {
            end_word
        } else {
            end_word - 1
        };

        for word_idx in first_full_word..=last_full_word {
            bitmap[word_idx] = u64::MAX;
        }
    }
}
