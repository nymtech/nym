// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! x86/x86_64 SIMD implementation of bitmap operations.
//! Provides optimized implementations using SSE2 and AVX2 intrinsics.

use super::BitmapOps;

// Track execution counts for debugging
static mut AVX2_CLEAR_COUNT: usize = 0;
static mut SSE2_CLEAR_COUNT: usize = 0;
static mut SCALAR_CLEAR_COUNT: usize = 0;

// Import the appropriate SIMD intrinsics
#[cfg(target_feature = "avx2")]
use std::arch::x86_64::{
    __m256i, _mm256_cmpeq_epi64, _mm256_load_si256, _mm256_loadu_si256, _mm256_movemask_epi8,
    _mm256_or_si256, _mm256_set1_epi64x, _mm256_setzero_si256, _mm256_store_si256,
    _mm256_storeu_si256, _mm256_testz_si256,
};

#[cfg(target_feature = "sse2")]
use std::arch::x86_64::{
    __m128i, _mm_cmpeq_epi64, _mm_loadu_si128, _mm_or_si128, _mm_set1_epi64x, _mm_setzero_si128,
    _mm_storeu_si128, _mm_testz_si128,
};

#[cfg(all(target_feature = "sse2", not(target_feature = "sse4.1")))]
use std::arch::x86_64::{_mm_cmpeq_epi64, _mm_movemask_epi8};

/// x86/x86_64 SIMD bitmap operations implementation
pub struct X86BitmapOps;

impl BitmapOps for X86BitmapOps {
    #[allow(unreachable_code)]
    #[inline(always)]
    fn clear_words(bitmap: &mut [u64], start_idx: usize, num_words: usize) {
        debug_assert!(start_idx + num_words <= bitmap.len());

        // First try AVX2 (256-bit, 4 words at a time)
        #[cfg(target_feature = "avx2")]
        unsafe {
            // Track execution count
            AVX2_CLEAR_COUNT += 1;

            // Process 4 words at a time with AVX2
            let zero_vec = _mm256_setzero_si256();
            let mut idx = start_idx;
            let end_idx = start_idx + num_words;

            // Process aligned blocks of 4 words
            while idx + 4 <= end_idx {
                // Safety:
                // - bitmap[idx..] is valid for reads/writes of at least 4 u64 words (32 bytes)
                // - We've validated with the debug_assert that start_idx + num_words <= bitmap.len()
                // - We check that idx + 4 <= end_idx to ensure we have 4 complete words
                // - The unaligned _storeu_ variant is used to handle any alignment
                _mm256_storeu_si256(bitmap[idx..].as_mut_ptr() as *mut __m256i, zero_vec);
                idx += 4;
            }

            // Handle remaining words with SSE2 or scalar ops
            if idx < end_idx {
                if idx + 2 <= end_idx {
                    // Use SSE2 for 2 words
                    // Safety: Same as above, but for 2 words (16 bytes) instead of 4
                    let sse_zero = _mm_setzero_si128();
                    _mm_storeu_si128(bitmap[idx..].as_mut_ptr() as *mut __m128i, sse_zero);
                    idx += 2;
                }

                // Handle any remaining words
                while idx < end_idx {
                    bitmap[idx] = 0;
                    idx += 1;
                }
            }

            return;
        }

        // If AVX2 is unavailable, try SSE2 (128-bit, 2 words at a time)
        #[cfg(all(target_feature = "sse2", not(target_feature = "avx2")))]
        unsafe {
            // Track execution count
            SSE2_CLEAR_COUNT += 1;

            // Process 2 words at a time with SSE2
            let zero_vec = _mm_setzero_si128();
            let mut idx = start_idx;
            let end_idx = start_idx + num_words;

            // Process aligned blocks of 2 words
            while idx + 2 <= end_idx {
                // Safety:
                // - bitmap[idx..] is valid for reads/writes of at least 2 u64 words (16 bytes)
                // - We've validated with the debug_assert that start_idx + num_words <= bitmap.len()
                // - We check that idx + 2 <= end_idx to ensure we have 2 complete words
                // - The unaligned _storeu_ variant is used to handle any alignment
                _mm_storeu_si128(bitmap[idx..].as_mut_ptr() as *mut __m128i, zero_vec);
                idx += 2;
            }

            // Handle remaining word (if any)
            if idx < end_idx {
                bitmap[idx] = 0;
            }

            return;
        }

        // Fallback to scalar implementation if no SIMD features available
        unsafe {
            // Safety: Just increments a static counter, with no possibility of data races
            // as long as this function isn't called concurrently
            SCALAR_CLEAR_COUNT += 1;
        }

        // Scalar fallback
        for i in start_idx..(start_idx + num_words) {
            bitmap[i] = 0;
        }
    }

    #[allow(unreachable_code)]
    #[inline(always)]
    fn is_range_zero(bitmap: &[u64], start_idx: usize, num_words: usize) -> bool {
        debug_assert!(start_idx + num_words <= bitmap.len());

        // First try AVX2 (256-bit, 4 words at a time)
        #[cfg(target_feature = "avx2")]
        unsafe {
            let mut idx = start_idx;
            let end_idx = start_idx + num_words;

            // Process aligned blocks of 4 words
            while idx + 4 <= end_idx {
                // Safety:
                // - bitmap[idx..] is valid for reads of at least 4 u64 words (32 bytes)
                // - We've validated with the debug_assert that start_idx + num_words <= bitmap.len()
                // - We check that idx + 4 <= end_idx to ensure we have 4 complete words
                // - The unaligned _loadu_ variant is used to handle any alignment
                let data_vec = _mm256_loadu_si256(bitmap[idx..].as_ptr() as *const __m256i);

                // Check if any bits are non-zero
                // Safety: _mm256_testz_si256 is safe when given valid __m256i values,
                // which data_vec is guaranteed to be
                if !_mm256_testz_si256(data_vec, data_vec) {
                    return false;
                }

                idx += 4;
            }

            // Handle remaining words with SSE2 or scalar ops
            if idx < end_idx {
                if idx + 2 <= end_idx {
                    // Use SSE2 for 2 words
                    // Safety:
                    // - bitmap[idx..] is valid for reads of at least 2 u64 words (16 bytes)
                    // - We check that idx + 2 <= end_idx to ensure we have 2 complete words
                    let data_vec = _mm_loadu_si128(bitmap[idx..].as_ptr() as *const __m128i);

                    // Safety: _mm_testz_si128 is safe when given valid __m128i values
                    if !_mm_testz_si128(data_vec, data_vec) {
                        return false;
                    }
                    idx += 2;
                }

                // Handle any remaining words
                while idx < end_idx {
                    if bitmap[idx] != 0 {
                        return false;
                    }
                    idx += 1;
                }
            }

            return true;
        }

        // If AVX2 is unavailable, try SSE2 (128-bit, 2 words at a time)
        #[cfg(all(target_feature = "sse2", not(target_feature = "avx2")))]
        unsafe {
            let mut idx = start_idx;
            let end_idx = start_idx + num_words;

            // Process aligned blocks of 2 words
            while idx + 2 <= end_idx {
                // Safety:
                // - bitmap[idx..] is valid for reads of at least 2 u64 words (16 bytes)
                // - We've validated with the debug_assert that start_idx + num_words <= bitmap.len()
                // - We check that idx + 2 <= end_idx to ensure we have 2 complete words
                // - The unaligned _loadu_ variant is used to handle any alignment
                let data_vec = _mm_loadu_si128(bitmap[idx..].as_ptr() as *const __m128i);

                // Check if any bits are non-zero (SSE4.1 would have _mm_testz_si128,
                // but for SSE2 compatibility we need to use a different approach)
                #[cfg(target_feature = "sse4.1")]
                {
                    // Safety: _mm_testz_si128 is safe when given valid __m128i values
                    if !_mm_testz_si128(data_vec, data_vec) {
                        return false;
                    }
                }

                #[cfg(not(target_feature = "sse4.1"))]
                {
                    // Compare with zero vector using SSE2 only
                    // Safety: All operations are valid with the data_vec value
                    let zero_vec = _mm_setzero_si128();
                    let cmp = _mm_cmpeq_epi64(data_vec, zero_vec);

                    // The movemask gives us a bit for each byte, set if the high bit of the byte is set
                    // For all-zero comparison, all 16 bits should be set (0xFFFF)
                    let mask = _mm_movemask_epi8(cmp);
                    if mask != 0xFFFF {
                        return false;
                    }
                }

                idx += 2;
            }

            // Handle remaining word (if any)
            if idx < end_idx && bitmap[idx] != 0 {
                return false;
            }

            return true;
        }

        // Scalar fallback
        bitmap[start_idx..(start_idx + num_words)]
            .iter()
            .all(|&word| word == 0)
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

/// Additional x86 optimized operations not covered by the trait
pub mod atomic {
    use super::*;

    /// Check and set bit, returning the previous state
    /// This function is not actually atomic! It's just a non-atomic optimization
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

    /// Set multiple bits at once using SIMD when possible
    ///
    /// # Safety
    ///
    /// This function is unsafe because it:
    /// - Uses SIMD intrinsics that require the AVX2 CPU feature to be available
    /// - Accesses bitmap memory through raw pointers
    /// - Does not perform bounds checking beyond what's required for SIMD operations
    ///
    /// Caller must ensure:
    /// - The AVX2 feature is available on the current CPU
    /// - `bitmap` has sufficient size to hold indices up to `end_bit/64`
    /// - `start_bit` and `end_bit` are valid bit indices within the bitmap
    /// - No other thread is concurrently modifying the same memory
    #[inline(always)]
    #[cfg(target_feature = "avx2")]
    pub unsafe fn set_bits_range(bitmap: &mut [u64], start_bit: u64, end_bit: u64) {
        // Process whole words where possible
        let start_word = (start_bit / 64) as usize;
        let end_word = (end_bit / 64) as usize;

        // Special case: all bits in the same word
        if start_word == end_word {
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

        // Handle complete words in the middle using AVX2
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

        if first_full_word <= last_full_word {
            // Use AVX2 to set multiple words at once
            // Safety: _mm256_set1_epi64x is safe to call with any i64 value
            let ones = _mm256_set1_epi64x(-1); // All bits set to 1

            let mut i = first_full_word;
            while i + 4 <= last_full_word + 1 {
                // Safety:
                // - bitmap[i..] is valid for reads/writes of at least 4 u64 words (32 bytes)
                // - We check that i + 4 <= last_full_word + 1 to ensure we have 4 complete words
                // - The unaligned _loadu/_storeu variants are used to handle any alignment
                let current = _mm256_loadu_si256(bitmap[i..].as_ptr() as *const __m256i);
                let result = _mm256_or_si256(current, ones);
                _mm256_storeu_si256(bitmap[i..].as_mut_ptr() as *mut __m256i, result);
                i += 4;
            }

            // Use SSE2 for remaining pairs of words
            if i + 2 <= last_full_word + 1 {
                // Safety:
                // - bitmap[i..] is valid for reads/writes of at least 2 u64 words (16 bytes)
                // - We check that i + 2 <= last_full_word + 1 to ensure we have 2 complete words
                // - The unaligned _loadu/_storeu variants are used to handle any alignment
                let sse_ones = _mm_set1_epi64x(-1);
                let current = _mm_loadu_si128(bitmap[i..].as_ptr() as *const __m128i);
                let result = _mm_or_si128(current, sse_ones);
                _mm_storeu_si128(bitmap[i..].as_mut_ptr() as *mut __m128i, result);
                i += 2;
            }

            // Handle any remaining words
            while i <= last_full_word {
                bitmap[i] = u64::MAX;
                i += 1;
            }
        }
    }

    /// Set multiple bits at once using SSE2 (when AVX2 not available)
    ///
    /// # Safety
    ///
    /// This function is unsafe because it:
    /// - Uses SIMD intrinsics that require the SSE2 CPU feature to be available
    /// - Accesses bitmap memory through raw pointers
    /// - Does not perform bounds checking beyond what's required for SIMD operations
    ///
    /// Caller must ensure:
    /// - The SSE2 feature is available on the current CPU
    /// - `bitmap` has sufficient size to hold indices up to `end_bit/64`
    /// - `start_bit` and `end_bit` are valid bit indices within the bitmap
    /// - No other thread is concurrently modifying the same memory
    #[inline(always)]
    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2")))]
    pub unsafe fn set_bits_range(bitmap: &mut [u64], start_bit: u64, end_bit: u64) {
        // Process whole words where possible
        let start_word = (start_bit / 64) as usize;
        let end_word = (end_bit / 64) as usize;

        // Special case: all bits in the same word
        if start_word == end_word {
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

        // Handle complete words in the middle using SSE2
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

        if first_full_word <= last_full_word {
            // Use SSE2 to set multiple words at once
            // Safety: _mm_set1_epi64x is safe to call with any i64 value
            let ones = unsafe { _mm_set1_epi64x(-1) }; // All bits set to 1

            let mut i = first_full_word;
            while i + 2 <= last_full_word + 1 {
                // Safety:
                // - bitmap[i..] is valid for reads/writes of at least 2 u64 words (16 bytes)
                // - We check that i + 2 <= last_full_word + 1 to ensure we have 2 complete words
                // - The unaligned _loadu/_storeu variants are used to handle any alignment
                let current = _mm_loadu_si128(bitmap[i..].as_ptr() as *const __m128i);
                let result = unsafe { _mm_or_si128(current, ones) };
                unsafe { _mm_storeu_si128(bitmap[i..].as_mut_ptr() as *mut __m128i, result) };
                i += 2;
            }

            // Handle any remaining words
            while i <= last_full_word {
                bitmap[i] = u64::MAX;
                i += 1;
            }
        }
    }

    /// Set multiple bits at once using scalar operations (fallback)
    #[inline(always)]
    #[cfg(not(any(target_feature = "avx2", target_feature = "sse2")))]
    pub fn set_bits_range(bitmap: &mut [u64], start_bit: u64, end_bit: u64) {
        // Process whole words where possible
        let start_word = (start_bit / 64) as usize;
        let end_word = (end_bit / 64) as usize;

        // Special case: all bits in the same word
        if start_word == end_word {
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

        for i in first_full_word..=last_full_word {
            bitmap[i] = u64::MAX;
        }
    }
}
