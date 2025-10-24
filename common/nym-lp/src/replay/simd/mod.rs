// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! SIMD optimizations for the replay protection bitmap operations.
//!
//! This module provides architecture-specific SIMD implementations with a common interface.

// Re-export the appropriate implementation
#[cfg(target_arch = "x86_64")]
mod x86;
#[cfg(target_arch = "x86_64")]
pub use self::x86::*;

#[cfg(target_arch = "aarch64")]
mod arm;
#[cfg(target_arch = "aarch64")]
pub use self::arm::*;

// Fallback scalar implementation for all other architectures
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod scalar;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub use self::scalar::*;

/// Trait defining SIMD operations for bitmap manipulation
pub trait BitmapOps {
    /// Clear a range of words in the bitmap
    fn clear_words(bitmap: &mut [u64], start_idx: usize, num_words: usize);

    /// Check if a range of words in the bitmap is all zeros
    fn is_range_zero(bitmap: &[u64], start_idx: usize, num_words: usize) -> bool;

    /// Set a specific bit in the bitmap
    fn set_bit(bitmap: &mut [u64], bit_idx: u64);

    /// Clear a specific bit in the bitmap
    fn clear_bit(bitmap: &mut [u64], bit_idx: u64);

    /// Check if a specific bit is set in the bitmap
    fn check_bit(bitmap: &[u64], bit_idx: u64) -> bool;
}

/// Get the optimal number of words to process in a SIMD operation
/// for the current architecture
#[inline(always)]
pub fn optimal_simd_width() -> usize {
    // This value is specialized for each architecture in their respective modules
    OPTIMAL_SIMD_WIDTH
}

/// Constant indicating the optimal SIMD processing width in number of u64 words
/// for the current architecture
#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
pub const OPTIMAL_SIMD_WIDTH: usize = 4; // 256 bits = 4 u64 words

#[cfg(target_arch = "x86_64")]
#[cfg(all(not(target_feature = "avx2"), target_feature = "sse2"))]
pub const OPTIMAL_SIMD_WIDTH: usize = 2; // 128 bits = 2 u64 words

#[cfg(target_arch = "aarch64")]
#[cfg(target_feature = "neon")]
pub const OPTIMAL_SIMD_WIDTH: usize = 2; // 128 bits = 2 u64 words

// Fallback for non-SIMD platforms or when features aren't available
#[cfg(not(any(
    all(target_arch = "x86_64", target_feature = "avx2"),
    all(target_arch = "x86_64", target_feature = "sse2"),
    all(target_arch = "aarch64", target_feature = "neon")
)))]
pub const OPTIMAL_SIMD_WIDTH: usize = 1; // Scalar fallback
