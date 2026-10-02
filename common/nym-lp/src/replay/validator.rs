// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Replay protection validator implementation.
//!
//! This module implements the core replay protection logic using a bitmap-based
//! approach to track received packets and validate their sequence.

use crate::replay::error::{ReplayError, ReplayResult};

/// Size of a word in the bitmap (64 bits)
const WORD_SIZE: usize = 64;

/// Default replay window size in bits; same as wireguard's `COUNTER_BITS_TOTAL`.
pub const DEFAULT_WINDOW_BITS: usize = 8192;

/// Current packet count statistics
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PacketCount {
    /// the next expected counter value
    pub next: u64,

    /// the total number of received packets
    pub received: u64,
}

/// Validator for receiving key counters to prevent replay attacks.
///
/// This structure maintains a bitmap of received packets and validates
/// incoming packet counters to ensure they are not replayed.
/// The size of the reordering window is configurable and defaults to
/// [`DEFAULT_WINDOW_BITS`].
#[derive(Debug, Clone)]
pub struct ReceivingKeyCounterValidator {
    /// Next expected counter value
    next: u64,

    /// Total number of received packets
    receive_cnt: u64,

    /// Bitmap for tracking received packets
    bitmap: Box<[u64]>,
}

impl Default for ReceivingKeyCounterValidator {
    fn default() -> Self {
        Self::new(0)
    }
}

impl ReceivingKeyCounterValidator {
    /// Creates a new validator with the given initial counter value and the default window size.
    pub fn new(initial_counter: u64) -> Self {
        Self::with_initial_counter_and_window(initial_counter, DEFAULT_WINDOW_BITS)
    }

    /// Creates a new validator with the given window size, rounded up to a whole number of words.
    pub fn with_window_bits(window_bits: usize) -> Self {
        Self::with_initial_counter_and_window(0, window_bits)
    }

    fn with_initial_counter_and_window(initial_counter: u64, window_bits: usize) -> Self {
        let n_words = window_bits.div_ceil(WORD_SIZE).max(1);
        Self {
            next: initial_counter,
            receive_cnt: 0,
            bitmap: vec![0; n_words].into_boxed_slice(),
        }
    }

    /// Returns the size of the replay window in bits.
    pub fn window_bits(&self) -> usize {
        self.bitmap.len() * WORD_SIZE
    }

    /// Returns the size of the replay window as a u64 for counter arithmetic.
    #[inline(always)]
    fn n_bits(&self) -> u64 {
        (self.bitmap.len() * WORD_SIZE) as u64
    }

    /// Sets a bit in the bitmap to mark a counter as received.
    #[inline(always)]
    fn set_bit(&mut self, idx: u64) {
        let bit_idx = idx % self.n_bits();

        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        self.bitmap[word_idx] |= 1u64 << bit_pos;
    }

    /// Clears a bit in the bitmap.
    #[inline(always)]
    fn clear_bit(&mut self, idx: u64) {
        let bit_idx = idx % self.n_bits();

        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        self.bitmap[word_idx] &= !(1u64 << bit_pos);
    }

    /// Returns true if the bit is set, false otherwise.
    #[inline(always)]
    fn check_bit(&self, idx: u64) -> bool {
        let bit_idx = idx % self.n_bits();

        let word_idx = (bit_idx / 64) as usize;
        let bit_pos = bit_idx % 64;
        (self.bitmap[word_idx] & (1u64 << bit_pos)) != 0
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
        let n_bits = self.n_bits();
        let too_far_back = if counter > u64::MAX - n_bits {
            // If adding the window size would overflow, it can't be too far back
            false
        } else {
            counter + n_bits < self.next
        };

        let duplicate = self.check_bit(counter);

        if is_growing {
            Ok(())
        } else if too_far_back {
            Err(ReplayError::OutOfWindow)
        } else if duplicate {
            Err(ReplayError::DuplicateCounter)
        } else {
            Ok(())
        }
    }

    /// Checks if the bitmap is completely empty (all zeros).
    /// Used for the fast-path optimisation when the whole window can be skipped.
    #[inline(always)]
    fn is_bitmap_empty(&self) -> bool {
        self.bitmap.iter().all(|&word| word == 0)
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
        let n_bits = self.n_bits();
        let too_far_back = if counter > u64::MAX - n_bits {
            // If adding the window size would overflow, it can't be too far back
            false
        } else {
            counter + n_bits < self.next
        };

        let is_sequential = counter == self.next;
        let is_out_of_order = counter < self.next;

        // Early return for out-of-window condition
        if too_far_back {
            return Err(ReplayError::OutOfWindow);
        }

        // Check for duplicate (only matters for out-of-order packets)
        let duplicate = is_out_of_order && self.check_bit(counter);
        if duplicate {
            return Err(ReplayError::DuplicateCounter);
        }

        // Fast path for far ahead counters with empty bitmap
        let far_ahead = counter.saturating_sub(self.next) >= n_bits;
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
    /// Returns a struct consisting of `(next, receive_cnt)` where:
    /// - `next` is the next expected counter value
    /// - `receive_cnt` is the total number of received packets
    pub fn current_packet_cnt(&self) -> PacketCount {
        PacketCount {
            next: self.next,
            received: self.receive_cnt,
        }
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

    /// Clears every tracked bit in the half-open range `[next, counter)` as the
    /// window slides forward to `counter`.
    #[inline(always)]
    fn clear_window(&mut self, counter: u64) {
        // Fast path: the jump spans at least a full window, so every tracked bit
        // is now out of range - clear the whole bitmap in one go.
        if counter.saturating_sub(self.next) >= self.n_bits() {
            self.bitmap.fill(0);
            return;
        }

        let mut i = self.next;

        // Leading partial word, bit by bit, up to the first word boundary.
        while !i.is_multiple_of(WORD_SIZE as u64) && i < counter {
            self.clear_bit(i);
            i += 1;
        }

        // Whole words; `i` is word-aligned here.
        while counter.saturating_sub(i) >= WORD_SIZE as u64 {
            let word = (i % self.n_bits() / WORD_SIZE as u64) as usize;
            self.bitmap[word] = 0;
            i += WORD_SIZE as u64;
        }

        // Trailing partial word, bit by bit.
        while i < counter {
            self.clear_bit(i);
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Most tests below were written in terms of the window size
    const N_BITS: usize = DEFAULT_WINDOW_BITS;

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
        assert!(
            validator
                .mark_did_receive_branchless(1000 + 72 + 125)
                .is_ok()
        );
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
        let PacketCount {
            next,
            received: count,
        } = validator.current_packet_cnt();
        assert_eq!(next, 0);
        assert_eq!(count, 0);

        // After receiving some packets
        assert!(validator.mark_did_receive_branchless(0).is_ok());
        assert!(validator.mark_did_receive_branchless(1).is_ok());
        assert!(validator.mark_did_receive_branchless(2).is_ok());

        let PacketCount {
            next,
            received: count,
        } = validator.current_packet_cnt();
        assert_eq!(next, 3);
        assert_eq!(count, 3);

        // After an out of order packet
        assert!(validator.mark_did_receive_branchless(10).is_ok());

        let PacketCount {
            next,
            received: count,
        } = validator.current_packet_cnt();
        assert_eq!(next, 11);
        assert_eq!(count, 4);

        // After a packet from the past (within window)
        assert!(validator.mark_did_receive_branchless(5).is_ok());

        let PacketCount {
            next,
            received: count,
        } = validator.current_packet_cnt();
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
        let first_jump = (N_BITS as u64) * 2;
        assert!(validator.mark_did_receive_branchless(first_jump).is_ok());

        // Verify next counter is updated
        let PacketCount { next, .. } = validator.current_packet_cnt();
        assert_eq!(next, first_jump + 1);

        // Second large jump, even further ahead
        let second_jump = first_jump + (N_BITS as u64) * 3;
        assert!(validator.mark_did_receive_branchless(second_jump).is_ok());

        // Verify next counter is updated again
        let PacketCount { next, .. } = validator.current_packet_cnt();
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
        let jump = (N_BITS as u64) * 2;
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
        let count = final_state.current_packet_cnt();

        // Verify that the received count matches our successful operations
        assert_eq!(count.received, total_successes as u64);
    }

    #[test]
    fn test_window_sizes() {
        // the default window matches wireguard's COUNTER_BITS_TOTAL
        let validator = ReceivingKeyCounterValidator::default();
        assert_eq!(validator.window_bits(), DEFAULT_WINDOW_BITS);
        assert_eq!(validator.window_bits(), 8192);

        // requested sizes are rounded up to whole words
        for (requested, expected) in [(0, 64), (1, 64), (64, 64), (65, 128), (1024, 1024)] {
            let validator = ReceivingKeyCounterValidator::with_window_bits(requested);
            assert_eq!(validator.window_bits(), expected);
            assert_eq!(validator.bitmap.len(), expected / WORD_SIZE);
        }
    }

    #[test]
    fn test_custom_window_size() {
        let mut validator = ReceivingKeyCounterValidator::with_window_bits(128);

        assert!(validator.mark_did_receive_branchless(0).is_ok());
        assert!(validator.mark_did_receive_branchless(300).is_ok());

        // the window is now [173, 300]: everything below is gone for good
        assert!(matches!(
            validator.will_accept_branchless(172),
            Err(ReplayError::OutOfWindow)
        ));
        assert!(validator.mark_did_receive_branchless(172).is_err());

        assert!(validator.will_accept_branchless(173).is_ok());
        assert!(validator.mark_did_receive_branchless(173).is_ok());
        assert!(matches!(
            validator.mark_did_receive_branchless(173),
            Err(ReplayError::DuplicateCounter)
        ));
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
