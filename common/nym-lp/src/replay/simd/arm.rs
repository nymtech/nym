// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! ARM NEON implementation of bitmap operations.

use super::BitmapOps;

#[cfg(target_feature = "neon")]
use std::arch::aarch64::{vceqq_u64, vdupq_n_u64, vgetq_lane_u64, vld1q_u64, vst1q_u64};

/// ARM NEON bitmap operations implementation
pub struct ArmBitmapOps;

impl BitmapOps for ArmBitmapOps {
    #[inline(always)]
    fn clear_words(bitmap: &mut [u64], start_idx: usize, num_words: usize) {
        debug_assert!(start_idx + num_words <= bitmap.len());

        #[cfg(target_feature = "neon")]
        unsafe {
            // Process 2 words at a time with NEON
            // Safety:
            // - vdupq_n_u64 is safe to call with any u64 value
            let zero_vec = vdupq_n_u64(0);
            let mut idx = start_idx;
            let end_idx = start_idx + num_words;

            // Process aligned blocks of 2 words
            while idx + 2 <= end_idx {
                // Safety:
                // - bitmap[idx..] is valid for reads/writes of at least 2 u64 words (16 bytes)
                // - We've validated with the debug_assert that start_idx + num_words <= bitmap.len()
                // - We check that idx + 2 <= end_idx to ensure we have 2 complete words
                vst1q_u64(bitmap[idx..].as_mut_ptr(), zero_vec);
                idx += 2;
            }

            // Handle remaining words (0 or 1)
            while idx < end_idx {
                bitmap[idx] = 0;
                idx += 1;
            }
        }

        #[cfg(not(target_feature = "neon"))]
        {
            // Fallback to scalar implementation
            for i in start_idx..(start_idx + num_words) {
                bitmap[i] = 0;
            }
        }
    }

    #[inline(always)]
    fn is_range_zero(bitmap: &[u64], start_idx: usize, num_words: usize) -> bool {
        debug_assert!(start_idx + num_words <= bitmap.len());

        #[cfg(target_feature = "neon")]
        unsafe {
            // Process 2 words at a time with NEON
            // Safety:
            // - vdupq_n_u64 is safe to call with any u64 value
            let zero_vec = vdupq_n_u64(0);
            let mut idx = start_idx;
            let end_idx = start_idx + num_words;

            // Process aligned blocks of 2 words
            while idx + 2 <= end_idx {
                // Safety:
                // - bitmap[idx..] is valid for reads of at least 2 u64 words (16 bytes)
                // - We've validated with the debug_assert that start_idx + num_words <= bitmap.len()
                // - We check that idx + 2 <= end_idx to ensure we have 2 complete words
                let data_vec = vld1q_u64(bitmap[idx..].as_ptr());

                // Safety:
                // - vceqq_u64 is safe when given valid vector values from vld1q_u64 and vdupq_n_u64
                // - vgetq_lane_u64 is safe with valid indices (0 and 1) for a 2-lane vector
                let cmp_result = vceqq_u64(data_vec, zero_vec);
                let mask1 = vgetq_lane_u64(cmp_result, 0);
                let mask2 = vgetq_lane_u64(cmp_result, 1);

                if (mask1 & mask2) != u64::MAX {
                    return false;
                }

                idx += 2;
            }

            // Handle remaining words (0 or 1)
            while idx < end_idx {
                if bitmap[idx] != 0 {
                    return false;
                }
                idx += 1;
            }

            true
        }

        #[cfg(not(target_feature = "neon"))]
        {
            // Fallback to scalar implementation
            bitmap[start_idx..(start_idx + num_words)]
                .iter()
                .all(|&w| w == 0)
        }
    }

    #[inline(always)]
    fn set_bit(bitmap: &mut [u64], bit_idx: u64) {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        bitmap[word_idx] |= 1u64 << bit_pos;
    }

    #[inline(always)]
    fn clear_bit(bitmap: &mut [u64], bit_idx: u64) {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        bitmap[word_idx] &= !(1u64 << bit_pos);
    }

    #[inline(always)]
    fn check_bit(bitmap: &[u64], bit_idx: u64) -> bool {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        (bitmap[word_idx] & (1u64 << bit_pos)) != 0
    }
}

/// We also implement optimized versions for specific operations that could
/// benefit from NEON but don't fit the general trait pattern
///
/// Atomic operations for the bitmap
pub mod atomic {
    #[cfg(target_feature = "neon")]
    use std::arch::aarch64::{vdupq_n_u64, vld1q_u64, vorrq_u64, vst1q_u64};

    /// Check and set bit, returning the previous state
    /// This function is not actually atomic! It's just a non-atomic optimization
    /// For actual atomic operations, the caller must provide proper synchronization
    #[inline(always)]
    pub fn check_and_set_bit(bitmap: &mut [u64], bit_idx: u64) -> bool {
        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        let mask = 1u64 << bit_pos;

        // Get old value
        let old_word = bitmap[word_idx];

        // Set bit regardless of current state
        bitmap[word_idx] |= mask;

        // Return true if bit was already set (duplicate)
        (old_word & mask) != 0
    }

    /// Set a range of bits efficiently using NEON
    ///
    /// # Safety
    ///
    /// This function is unsafe because it:
    /// - Uses SIMD intrinsics that require the NEON CPU feature to be available
    /// - Accesses bitmap memory through raw pointers
    /// - Does not perform bounds checking beyond what's required for SIMD operations
    ///
    /// Caller must ensure:
    /// - The NEON feature is available on the current CPU
    /// - `bitmap` has sufficient size to hold indices up to `end_bit/64`
    /// - `start_bit` and `end_bit` are valid bit indices within the bitmap
    /// - No other thread is concurrently modifying the same memory
    #[inline(always)]
    #[cfg(target_feature = "neon")]
    pub unsafe fn set_bits_range(bitmap: &mut [u64], start_bit: u64, end_bit: u64) {
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
        if !start_bit.is_multiple_of(64) {
            let start_mask = u64::MAX << (start_bit % 64);
            bitmap[start_word] |= start_mask;
        }

        if !(end_bit + 1).is_multiple_of(64) {
            let end_mask = u64::MAX >> (63 - (end_bit % 64));
            bitmap[end_word] |= end_mask;
        }

        // Handle complete words in the middle using NEON
        let first_full_word = if start_bit.is_multiple_of(64) {
            start_word
        } else {
            start_word + 1
        };
        let last_full_word = if (end_bit + 1).is_multiple_of(64) {
            end_word
        } else {
            end_word - 1
        };

        if first_full_word <= last_full_word {
            // Use NEON to set words faster
            // Safety: vdupq_n_u64 is safe to call with any u64 value
            let ones_vec = unsafe { vdupq_n_u64(u64::MAX) };
            let mut idx = first_full_word;

            while idx + 2 <= last_full_word + 1 {
                // Safety:
                // - bitmap[idx..] is valid for reads/writes of at least 2 u64 words (16 bytes)
                // - We check that idx + 2 <= last_full_word + 1 to ensure we have 2 complete words
                unsafe {
                    let current_vec = vld1q_u64(bitmap[idx..].as_ptr());
                    // Safety: vorrq_u64 is safe when given valid vector values
                    let result_vec = vorrq_u64(current_vec, ones_vec);
                    vst1q_u64(bitmap[idx..].as_mut_ptr(), result_vec);
                }

                idx += 2;
            }

            // Handle remaining words
            while idx <= last_full_word {
                bitmap[idx] = u64::MAX;
                idx += 1;
            }
        }
    }

    /// Set a range of bits efficiently (scalar fallback)
    #[inline(always)]
    #[cfg(not(target_feature = "neon"))]
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
