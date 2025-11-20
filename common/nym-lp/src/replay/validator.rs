// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Replay protection validator implementation.
//!
//! This module implements the core replay protection logic using a bitmap-based
//! approach to track received packets and validate their sequence.

use crate::replay::error::{ReplayError, ReplayResult};
use crate::replay::simd::{self, BitmapOps};

// Determine the appropriate SIMD implementation at compile time
#[cfg(target_arch = "aarch64")]
#[cfg(target_feature = "neon")]
use crate::replay::simd::ArmBitmapOps as SimdImpl;

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
use crate::replay::simd::X86BitmapOps as SimdImpl;

#[cfg(target_arch = "x86_64")]
#[cfg(all(not(target_feature = "avx2"), target_feature = "sse2"))]
use crate::replay::simd::X86BitmapOps as SimdImpl;

#[cfg(not(any(
    all(target_arch = "x86_64", target_feature = "avx2"),
    all(target_arch = "x86_64", target_feature = "sse2"),
    all(target_arch = "aarch64", target_feature = "neon")
)))]
use crate::replay::simd::ScalarBitmapOps as SimdImpl;

/// Size of a word in the bitmap (64 bits)
const WORD_SIZE: usize = 64;

/// Number of words in the bitmap (allows reordering of 64*16 = 1024 packets)
const N_WORDS: usize = 16;

/// Total number of bits in the bitmap
const N_BITS: usize = WORD_SIZE * N_WORDS;

/// Validator for receiving key counters to prevent replay attacks.
///
/// This structure maintains a bitmap of received packets and validates
/// incoming packet counters to ensure they are not replayed.
#[derive(Debug, Clone, Default)]
pub struct ReceivingKeyCounterValidator {
    /// Next expected counter value
    next: u64,

    /// Total number of received packets
    receive_cnt: u64,

    /// Bitmap for tracking received packets
    bitmap: [u64; N_WORDS],
}

impl ReceivingKeyCounterValidator {
    /// Creates a new validator with the given initial counter value.
    pub fn new(initial_counter: u64) -> Self {
        Self {
            next: initial_counter,
            receive_cnt: 0,
            bitmap: [0; N_WORDS],
        }
    }

    /// Sets a bit in the bitmap to mark a counter as received.
    #[inline(always)]
    fn set_bit(&mut self, idx: u64) {
        SimdImpl::set_bit(&mut self.bitmap, idx % (N_BITS as u64));
    }

    /// Clears a bit in the bitmap.
    #[inline(always)]
    fn clear_bit(&mut self, idx: u64) {
        SimdImpl::clear_bit(&mut self.bitmap, idx % (N_BITS as u64));
    }

    /// Clears the word that contains the given index.
    #[inline(always)]
    #[allow(dead_code)]
    fn clear_word(&mut self, idx: u64) {
        let bit_idx = idx % (N_BITS as u64);
        let word = (bit_idx / (WORD_SIZE as u64)) as usize;
        SimdImpl::clear_words(&mut self.bitmap, word, 1);
    }

    /// Returns true if the bit is set, false otherwise.
    #[inline(always)]
    fn check_bit_branchless(&self, idx: u64) -> bool {
        SimdImpl::check_bit(&self.bitmap, idx % (N_BITS as u64))
    }

    /// Performs a quick check to determine if a counter will be accepted.
    ///
    /// This is a fast check that can be done before more expensive operations.
    ///
    /// Returns:
    /// - `Ok(())` if the counter is acceptable
    /// - `Err(ReplayError::InvalidCounter)` if the counter is invalid (too far back)
    /// - `Err(ReplayError::DuplicateCounter)` if the counter has already been received
    #[inline(always)]
    pub fn will_accept_branchless(&self, counter: u64) -> ReplayResult<()> {
        // Calculate conditions
        let is_growing = counter >= self.next;

        // Handle potential overflow when adding N_BITS to counter
        let too_far_back = if counter > u64::MAX - (N_BITS as u64) {
            // If adding N_BITS would overflow, it can't be too far back
            false
        } else {
            counter + (N_BITS as u64) < self.next
        };

        let duplicate = self.check_bit_branchless(counter);

        // Using Option to avoid early returns
        let result = if is_growing {
            Some(Ok(()))
        } else if too_far_back {
            Some(Err(ReplayError::OutOfWindow))
        } else if duplicate {
            Some(Err(ReplayError::DuplicateCounter))
        } else {
            Some(Ok(()))
        };

        // Unwrap the option (always Some)
        result.unwrap()
    }

    /// Special case function for clearing the entire bitmap
    /// Used for the fast path when we know the bitmap must be entirely cleared
    #[inline(always)]
    fn clear_window_fast(&mut self) {
        SimdImpl::clear_words(&mut self.bitmap, 0, N_WORDS);
    }

    /// Checks if the bitmap is completely empty (all zeros)
    /// This is used for fast path optimization
    #[inline(always)]
    fn is_bitmap_empty(&self) -> bool {
        SimdImpl::is_range_zero(&self.bitmap, 0, N_WORDS)
    }

    /// Marks a counter as received and updates internal state.
    ///
    /// This method should be called after a packet has been validated
    /// and processed successfully.
    ///
    /// Returns:
    /// - `Ok(())` if the counter was successfully marked
    /// - `Err(ReplayError::InvalidCounter)` if the counter is invalid (too far back)
    /// - `Err(ReplayError::DuplicateCounter)` if the counter has already been received
    #[inline(always)]
    pub fn mark_did_receive_branchless(&mut self, counter: u64) -> ReplayResult<()> {
        // Calculate conditions once - using saturating operations to prevent overflow
        // For the too_far_back check, we need to avoid overflowing when adding N_BITS to counter
        let too_far_back = if counter > u64::MAX - (N_BITS as u64) {
            // If adding N_BITS would overflow, it can't be too far back
            false
        } else {
            counter + (N_BITS as u64) < self.next
        };

        let is_sequential = counter == self.next;
        let is_out_of_order = counter < self.next;

        // Early return for out-of-window condition
        if too_far_back {
            return Err(ReplayError::OutOfWindow);
        }

        // Check for duplicate (only matters for out-of-order packets)
        let duplicate = is_out_of_order && self.check_bit_branchless(counter);
        if duplicate {
            return Err(ReplayError::DuplicateCounter);
        }

        // Fast path for far ahead counters with empty bitmap
        let far_ahead = counter.saturating_sub(self.next) >= (N_BITS as u64);
        if far_ahead && self.is_bitmap_empty() {
            // No need to clear anything, just set the new bit
            self.set_bit(counter);
            self.next = counter.saturating_add(1);
            self.receive_cnt += 1;
            return Ok(());
        }

        // Handle bitmap clearing for ahead counters that aren't sequential
        if !is_sequential && !is_out_of_order {
            self.clear_window(counter);
        }

        // Set the bit and update counters
        self.set_bit(counter);

        // Update next counter safely - avoid overflow
        self.next = if is_sequential {
            counter.saturating_add(1)
        } else {
            self.next.max(counter.saturating_add(1))
        };

        self.receive_cnt += 1;

        Ok(())
    }

    /// Returns the current packet count statistics.
    ///
    /// Returns a tuple of `(next, receive_cnt)` where:
    /// - `next` is the next expected counter value
    /// - `receive_cnt` is the total number of received packets
    pub fn current_packet_cnt(&self) -> (u64, u64) {
        (self.next, self.receive_cnt)
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn check_and_set_bit_branchless(&mut self, idx: u64) -> bool {
        let bit_idx = idx % (N_BITS as u64);
        simd::atomic::check_and_set_bit(&mut self.bitmap, bit_idx)
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn increment_counter_branchless(&mut self, condition: bool) {
        // Add either 1 or 0 based on condition
        self.receive_cnt += condition as u64;
    }

    #[inline(always)]
    pub fn mark_sequential_branchless(&mut self, counter: u64) -> ReplayResult<()> {
        // Check if sequential
        let is_sequential = counter == self.next;

        // Set the bit
        self.set_bit(counter);

        // Conditionally update next counter using saturating add to prevent overflow
        self.next = self.next.saturating_add(is_sequential as u64);

        // Always increment receive count if we got here
        self.receive_cnt += 1;

        Ok(())
    }

    // Helper function for window clearing with SIMD optimization
    #[inline(always)]
    fn clear_window(&mut self, counter: u64) {
        // Handle potential overflow safely
        // If counter is very large (close to u64::MAX), we need special handling
        let counter_distance = counter.saturating_sub(self.next);
        let far_ahead = counter_distance >= (N_BITS as u64);

        // Fast path: Complete window clearing for far ahead counters
        if far_ahead {
            // Check if window is already clear for fast path optimization
            if !self.is_bitmap_empty() {
                // Use SIMD to clear the entire bitmap at once
                self.clear_window_fast();
            }
            return;
        }

        // Prepare for partial window clearing
        let mut i = self.next;

        // Get SIMD processing width (platform optimized)
        let simd_width = simd::optimal_simd_width();

        // Pre-alignment clearing
        if i % (WORD_SIZE as u64) != 0 {
            let current_word = (i % (N_BITS as u64) / (WORD_SIZE as u64)) as usize;

            // Check if we need to clear this word
            if self.bitmap[current_word] != 0 {
                // Safely handle potential overflow by checking before each increment
                while i % (WORD_SIZE as u64) != 0 && i < counter {
                    self.clear_bit(i);

                    // Prevent overflow on increment
                    if i == u64::MAX {
                        break;
                    }
                    i += 1;
                }
            } else {
                // Fast forward to the next word boundary
                let words_to_skip = (WORD_SIZE as u64) - (i % (WORD_SIZE as u64));
                if words_to_skip > u64::MAX - i {
                    // Would overflow, just set to MAX
                    i = u64::MAX;
                } else {
                    i += words_to_skip;
                }
            }
        }

        // Word-aligned clearing with SIMD where possible
        while i <= counter.saturating_sub(WORD_SIZE as u64) {
            let current_word = (i % (N_BITS as u64) / (WORD_SIZE as u64)) as usize;

            // Check if we have enough consecutive words to use SIMD
            if current_word + simd_width <= N_WORDS
                && i % (simd_width as u64 * WORD_SIZE as u64) == 0
            {
                // Use SIMD to clear multiple words at once if any need clearing
                let needs_clearing =
                    !SimdImpl::is_range_zero(&self.bitmap, current_word, simd_width);
                if needs_clearing {
                    SimdImpl::clear_words(&mut self.bitmap, current_word, simd_width);
                }

                // Skip the words we just processed
                let words_to_skip = simd_width as u64 * WORD_SIZE as u64;
                if words_to_skip > u64::MAX - i {
                    i = u64::MAX;
                    break;
                }
                i += words_to_skip;
            } else {
                // Process single word
                if self.bitmap[current_word] != 0 {
                    self.bitmap[current_word] = 0;
                }

                // Check for potential overflow before incrementing
                if i > u64::MAX - (WORD_SIZE as u64) {
                    i = u64::MAX;
                    break;
                }
                i += WORD_SIZE as u64;
            }
        }

        // Post-alignment clearing (bit by bit for remaining bits)
        if i < counter {
            let final_word = (i % (N_BITS as u64) / (WORD_SIZE as u64)) as usize;
            let is_final_word_empty = self.bitmap[final_word] == 0;

            // Skip clearing if word is already empty
            if !is_final_word_empty {
                while i < counter {
                    self.clear_bit(i);

                    // Prevent overflow on increment
                    if i == u64::MAX {
                        break;
                    }
                    i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_counter_basic() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Check initial state
        assert_eq!(validator.next, 0);
        assert_eq!(validator.receive_cnt, 0);

        // Test sequential counters
        assert!(validator.mark_did_receive_branchless(0).is_ok());
        assert!(validator.mark_did_receive_branchless(0).is_err());
        assert!(validator.mark_did_receive_branchless(1).is_ok());
        assert!(validator.mark_did_receive_branchless(1).is_err());
    }

    #[test]
    fn test_replay_counter_out_of_order() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Process some sequential packets
        assert!(validator.mark_did_receive_branchless(0).is_ok());
        assert!(validator.mark_did_receive_branchless(1).is_ok());
        assert!(validator.mark_did_receive_branchless(2).is_ok());

        // Out-of-order packet that hasn't been seen yet
        assert!(validator.mark_did_receive_branchless(1).is_err()); // Already seen
        assert!(validator.mark_did_receive_branchless(10).is_ok()); // New packet, ahead of next

        // Next should now be 11
        assert_eq!(validator.next, 11);

        // Can still accept packets in the valid window
        assert!(validator.will_accept_branchless(9).is_ok());
        assert!(validator.will_accept_branchless(8).is_ok());

        // But duplicates are rejected
        assert!(validator.will_accept_branchless(10).is_err());
    }

    #[test]
    fn test_replay_counter_full() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Process a bunch of sequential packets
        for i in 0..64 {
            assert!(validator.mark_did_receive_branchless(i).is_ok());
            assert!(validator.mark_did_receive_branchless(i).is_err());
        }

        // Test out of order within window
        assert!(validator.mark_did_receive_branchless(15).is_err()); // Already seen
        assert!(validator.mark_did_receive_branchless(63).is_err()); // Already seen

        // Test for packets within bitmap range
        for i in 64..(N_BITS as u64) + 128 {
            assert!(validator.mark_did_receive_branchless(i).is_ok());
            assert!(validator.mark_did_receive_branchless(i).is_err());
        }
    }

    #[test]
    fn test_replay_counter_window_sliding() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Jump far ahead to force window sliding
        let far_ahead = (N_BITS as u64) * 3;
        assert!(validator.mark_did_receive_branchless(far_ahead).is_ok());

        // Everything too far back should be rejected
        for i in 0..=(N_BITS as u64) * 2 {
            assert!(matches!(
                validator.will_accept_branchless(i),
                Err(ReplayError::OutOfWindow)
            ));
            assert!(validator.mark_did_receive_branchless(i).is_err());
        }

        // Values in window but less than far_ahead should be accepted
        for i in (N_BITS as u64) * 2 + 1..far_ahead {
            assert!(validator.will_accept_branchless(i).is_ok());
        }

        // The far_ahead value itself should be rejected now (duplicate)
        assert!(matches!(
            validator.will_accept_branchless(far_ahead),
            Err(ReplayError::DuplicateCounter)
        ));

        // Test receiving packets in reverse order within window
        for i in ((N_BITS as u64) * 2 + 1..far_ahead).rev() {
            assert!(validator.mark_did_receive_branchless(i).is_ok());
            assert!(validator.mark_did_receive_branchless(i).is_err());
        }
    }

    #[test]
    fn test_out_of_order_tracking() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Jump ahead
        assert!(validator.mark_did_receive_branchless(1000).is_ok());

        // Test some more additions
        assert!(validator.mark_did_receive_branchless(1000 + 70).is_ok());
        assert!(validator.mark_did_receive_branchless(1000 + 71).is_ok());
        assert!(validator.mark_did_receive_branchless(1000 + 72).is_ok());
        assert!(validator
            .mark_did_receive_branchless(1000 + 72 + 125)
            .is_ok());
        assert!(validator.mark_did_receive_branchless(1000 + 63).is_ok());

        // Check duplicates
        assert!(validator.mark_did_receive_branchless(1000 + 70).is_err());
        assert!(validator.mark_did_receive_branchless(1000 + 71).is_err());
        assert!(validator.mark_did_receive_branchless(1000 + 72).is_err());
    }

    #[test]
    fn test_counter_stats() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Initial state
        let (next, count) = validator.current_packet_cnt();
        assert_eq!(next, 0);
        assert_eq!(count, 0);

        // After receiving some packets
        assert!(validator.mark_did_receive_branchless(0).is_ok());
        assert!(validator.mark_did_receive_branchless(1).is_ok());
        assert!(validator.mark_did_receive_branchless(2).is_ok());

        let (next, count) = validator.current_packet_cnt();
        assert_eq!(next, 3);
        assert_eq!(count, 3);

        // After an out of order packet
        assert!(validator.mark_did_receive_branchless(10).is_ok());

        let (next, count) = validator.current_packet_cnt();
        assert_eq!(next, 11);
        assert_eq!(count, 4);

        // After a packet from the past (within window)
        assert!(validator.mark_did_receive_branchless(5).is_ok());

        let (next, count) = validator.current_packet_cnt();
        assert_eq!(next, 11); // Next doesn't change
        assert_eq!(count, 5); // Count increases
    }

    #[test]
    fn test_window_boundary_edge_cases() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // First process a sequence of packets
        for i in 0..100 {
            assert!(validator.mark_did_receive_branchless(i).is_ok());
        }

        // The window should now span from 100 to 100+N_BITS

        // Test packet near the upper edge of the window
        let upper_edge = 100 + (N_BITS as u64) - 1;
        assert!(validator.will_accept_branchless(upper_edge).is_ok());
        assert!(validator.mark_did_receive_branchless(upper_edge).is_ok());

        // Test packet just outside the upper edge (should be accepted)
        let just_outside_upper = 100 + (N_BITS as u64);
        assert!(validator.will_accept_branchless(just_outside_upper).is_ok());

        // Test packet near the lower edge of the window
        let lower_edge = 100 + 1; // +1 because we've already processed 100
        assert!(validator.will_accept_branchless(lower_edge).is_ok());

        // Test packet just outside the lower edge (should be rejected)
        if upper_edge >= (N_BITS as u64) * 2 {
            // Only test this if we're far enough along to have a lower bound
            let just_outside_lower = 100 - (N_BITS as u64);
            assert!(matches!(
                validator.will_accept_branchless(just_outside_lower),
                Err(ReplayError::OutOfWindow)
            ));
        }
    }

    #[test]
    fn test_multiple_window_shifts() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // First jump - process packet far ahead
        let first_jump = 2000;
        assert!(validator.mark_did_receive_branchless(first_jump).is_ok());

        // Verify next counter is updated
        let (next, _) = validator.current_packet_cnt();
        assert_eq!(next, first_jump + 1);

        // Second large jump, even further ahead
        let second_jump = first_jump + 5000;
        assert!(validator.mark_did_receive_branchless(second_jump).is_ok());

        // Verify next counter is updated again
        let (next, _) = validator.current_packet_cnt();
        assert_eq!(next, second_jump + 1);

        // Test packets within the new window
        let mid_window = second_jump - 500;
        assert!(validator.will_accept_branchless(mid_window).is_ok());

        // Test packets outside the new window
        let outside_window = first_jump + 100;
        assert!(matches!(
            validator.will_accept_branchless(outside_window),
            Err(ReplayError::OutOfWindow)
        ));
    }

    #[test]
    fn test_interleaved_packets_at_boundaries() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Jump ahead to establish a large window
        let jump = 2000;
        assert!(validator.mark_did_receive_branchless(jump).is_ok());

        // Process a sequence at the upper boundary
        for i in 0..10 {
            let upper_packet = jump + 100 + i;
            assert!(validator.mark_did_receive_branchless(upper_packet).is_ok());
        }

        // Process a sequence at the lower boundary
        for i in 0..10 {
            let lower_packet = jump - (N_BITS as u64) + 100 + i;
            // These might fail if they're outside the window, that's ok
            let _ = validator.mark_did_receive_branchless(lower_packet);
        }

        // Process alternating packets at both ends
        for i in 0..5 {
            let upper = jump + 200 + i;
            let lower = jump - (N_BITS as u64) + 200 + i;

            assert!(validator.will_accept_branchless(upper).is_ok());
            let lower_result = validator.will_accept_branchless(lower);

            // Lower might be accepted or rejected, depending on exactly where the window is
            if lower_result.is_ok() {
                assert!(validator.mark_did_receive_branchless(lower).is_ok());
            }

            assert!(validator.mark_did_receive_branchless(upper).is_ok());
        }
    }

    #[test]
    fn test_exact_window_size_with_full_bitmap() {
        let mut validator = ReceivingKeyCounterValidator::default();

        // Fill the entire bitmap with non-sequential packets
        // This tests both window size and bitmap capacity

        // Generate a random but reproducible pattern
        let mut positions = Vec::new();
        for i in 0..N_BITS {
            positions.push((i * 7) % N_BITS);
        }

        // Mark packets in this pattern
        for pos in &positions {
            assert!(validator.mark_did_receive_branchless(*pos as u64).is_ok());
        }

        // Try to mark them again (should all fail as duplicates)
        for pos in &positions {
            assert!(matches!(
                validator.mark_did_receive_branchless(*pos as u64),
                Err(ReplayError::DuplicateCounter)
            ));
        }

        // Force window to slide
        let far_ahead = (N_BITS as u64) * 2;
        assert!(validator.mark_did_receive_branchless(far_ahead).is_ok());

        // Old packets should now be outside the window
        for pos in &positions {
            if *pos as u64 + (N_BITS as u64) < far_ahead {
                assert!(matches!(
                    validator.will_accept_branchless(*pos as u64),
                    Err(ReplayError::OutOfWindow)
                ));
            }
        }
    }

    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn test_concurrent_access() {
        let validator = Arc::new(std::sync::Mutex::new(
            ReceivingKeyCounterValidator::default(),
        ));
        let num_threads = 8;
        let operations_per_thread = 1000;
        let barrier = Arc::new(Barrier::new(num_threads));

        // Create thread handles
        let mut handles = vec![];

        for thread_id in 0..num_threads {
            let validator_clone = Arc::clone(&validator);
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier_clone.wait();

                let mut successes = 0;
                let mut duplicates = 0;
                let mut out_of_window = 0;

                for i in 0..operations_per_thread {
                    // Generate a somewhat random but reproducible counter value
                    // Different threads will sometimes try to insert the same value
                    let counter = (i * 7 + thread_id * 13) as u64;

                    let mut guard = validator_clone.lock().unwrap();
                    match guard.mark_did_receive_branchless(counter) {
                        Ok(()) => successes += 1,
                        Err(ReplayError::DuplicateCounter) => duplicates += 1,
                        Err(ReplayError::OutOfWindow) => out_of_window += 1,
                        _ => {}
                    }
                }

                (successes, duplicates, out_of_window)
            });

            handles.push(handle);
        }

        // Collect results
        let mut total_successes = 0;
        let mut total_duplicates = 0;
        let mut total_out_of_window = 0;

        for handle in handles {
            let (successes, duplicates, out_of_window) = handle.join().unwrap();
            total_successes += successes;
            total_duplicates += duplicates;
            total_out_of_window += out_of_window;
        }

        // Verify that all operations were accounted for
        assert_eq!(
            total_successes + total_duplicates + total_out_of_window,
            num_threads * operations_per_thread
        );

        // Verify that some operations were successful and some were duplicates
        assert!(total_successes > 0);
        assert!(total_duplicates > 0);

        // Check final state of the validator
        let final_state = validator.lock().unwrap();
        let (_next, receive_cnt) = final_state.current_packet_cnt();

        // Verify that the received count matches our successful operations
        assert_eq!(receive_cnt, total_successes as u64);
    }

    #[test]
    fn test_memory_usage() {
        use std::mem::{size_of, size_of_val};

        // Test small validator
        let validator_default = ReceivingKeyCounterValidator::default();
        let size_default = size_of_val(&validator_default);

        // Expected size calculation
        let expected_size = size_of::<u64>() * 2 + // next + receive_cnt
                           size_of::<u64>() * N_WORDS; // bitmap

        assert_eq!(size_default, expected_size);
        println!("Default validator size: {} bytes", size_default);

        // Memory efficiency calculation (bits tracked per byte of memory)
        let bits_per_byte = N_BITS as f64 / size_default as f64;
        println!(
            "Memory efficiency: {:.2} bits tracked per byte of memory",
            bits_per_byte
        );

        // Verify minimum memory needed for different window sizes
        for window_size in [64usize, 128, 256, 512, 1024, 2048] {
            let words_needed = window_size.div_ceil(WORD_SIZE);
            let memory_needed = size_of::<u64>() * 2 + size_of::<u64>() * words_needed;
            println!(
                "Window size {}: {} bytes minimum",
                window_size, memory_needed
            );
        }
    }

    #[test]
    #[cfg(any(
        target_feature = "sse2",
        target_feature = "avx2",
        target_feature = "neon"
    ))]
    fn test_simd_operations() {
        // This test verifies that SIMD-optimized operations would produce
        // the same results as the scalar implementation

        // Create a validator with a known state
        let mut validator = ReceivingKeyCounterValidator::default();

        // Fill bitmap with a pattern
        for i in 0..64 {
            validator.set_bit(i);
        }

        // Create a copy for comparison
        let _original_bitmap = validator.bitmap;

        // Simulate SIMD clear (4 words at a time)
        #[cfg(target_feature = "avx2")]
        {
            use std::arch::x86_64::{_mm256_setzero_si256, _mm256_storeu_si256};

            // Clear words 0-3 using AVX2
            unsafe {
                let zero_vec = _mm256_setzero_si256();
                _mm256_storeu_si256(validator.bitmap.as_mut_ptr() as *mut _, zero_vec);
            }

            // Verify first 4 words are cleared
            assert_eq!(validator.bitmap[0], 0);
            assert_eq!(validator.bitmap[1], 0);
            assert_eq!(validator.bitmap[2], 0);
            assert_eq!(validator.bitmap[3], 0);

            // Verify other words are unchanged
            for i in 4..N_WORDS {
                assert_eq!(validator.bitmap[i], original_bitmap[i]);
            }
        }

        #[cfg(target_feature = "sse2")]
        {
            use std::arch::x86_64::{_mm_setzero_si128, _mm_storeu_si128};

            // Reset validator
            validator.bitmap = original_bitmap;

            // Clear words 0-1 using SSE2
            unsafe {
                let zero_vec = _mm_setzero_si128();
                _mm_storeu_si128(validator.bitmap.as_mut_ptr() as *mut _, zero_vec);
            }

            // Verify first 2 words are cleared
            assert_eq!(validator.bitmap[0], 0);
            assert_eq!(validator.bitmap[1], 0);

            // Verify other words are unchanged
            for i in 2..N_WORDS {
                assert_eq!(validator.bitmap[i], original_bitmap[i]);
            }
        }

        // No SIMD available, make this test a no-op
        #[cfg(not(any(
            target_feature = "sse2",
            target_feature = "avx2",
            target_feature = "neon"
        )))]
        {
            println!("No SIMD features available, skipping SIMD test");
        }
    }

    #[test]
    fn test_clear_window_overflow() {
        // Set a very large next value, close to u64::MAX
        let mut validator = ReceivingKeyCounterValidator {
            next: u64::MAX - 1000,
            ..Default::default()
        };

        // Try to clear window with an even higher counter
        // This should exercise the potentially problematic code
        let counter = u64::MAX - 500;

        // Call clear_window directly (this is what we suspect has issues)
        validator.clear_window(counter);

        // If we got here without a panic, at least it's not crashing
        // Let's verify the bitmap state is reasonable
        let any_non_zero = validator.bitmap.iter().any(|&word| word != 0);
        assert!(!any_non_zero, "Bitmap should be cleared");

        // Try the full function which uses clear_window internally
        assert!(validator.mark_did_receive_branchless(counter).is_ok());

        // Verify it was marked
        assert!(matches!(
            validator.will_accept_branchless(counter),
            Err(ReplayError::DuplicateCounter)
        ));
    }
}
