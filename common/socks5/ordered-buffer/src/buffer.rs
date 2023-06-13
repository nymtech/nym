// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OrderedMessageError {
    #[error("received message with sequence number {received}, which is way higher than our current {current}")]
    MessageSequenceTooLarge { current: u64, received: u64 },

    #[error("received message with sequence number {received}, while we're already at {current}!")]
    MessageAlreadyReconstructed { current: u64, received: u64 },

    #[error("attempted to overwrite message at sequence {received}")]
    AttemptedToOverwriteSequence { received: u64 },
}

/// Stores messages and emits them in order.
///
/// Only contiguous messages with an index less than or equal to `next_index`
/// will be returned - this avoids returning gaps while we wait for the buffer
/// to fill up with the full sequence.
#[derive(Debug)]
pub struct OrderedMessageBuffer {
    next_sequence: u64,
    messages: BTreeMap<u64, Vec<u8>>,
}

/// Data returned from `OrderedMessageBuffer` on a successful read of gapless ordered data.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadContiguousData {
    pub data: Vec<u8>,
    pub last_sequence: u64,
}

const MAX_REASONABLE_OFFSET: u64 = 1000;

impl OrderedMessageBuffer {
    pub fn new() -> OrderedMessageBuffer {
        OrderedMessageBuffer {
            next_sequence: 0,
            messages: BTreeMap::new(),
        }
    }

    /// Writes a message to the buffer. messages are sort on insertion, so
    /// that later on multiple reads for incomplete sequences don't result in
    /// useless sort work.
    pub fn write(&mut self, sequence: u64, data: Vec<u8>) -> Result<(), OrderedMessageError> {
        // reject messages that have clearly malformed sequence
        if sequence > self.next_sequence + MAX_REASONABLE_OFFSET {
            return Err(OrderedMessageError::MessageSequenceTooLarge {
                current: self.next_sequence,
                received: sequence,
            });
        }

        if self.messages.contains_key(&sequence) {
            return Err(OrderedMessageError::AttemptedToOverwriteSequence { received: sequence });
        }

        if sequence < self.next_sequence {
            return Err(OrderedMessageError::MessageAlreadyReconstructed {
                current: self.next_sequence,
                received: sequence,
            });
        }

        trace!(
            "Writing message index: {} length {} to OrderedMessageBuffer.",
            sequence,
            data.len()
        );

        self.messages.insert(sequence, data);
        Ok(())
    }

    /// Checks whether the buffer contains enough contiguous regions to read until the specified target sequence.
    pub fn can_read_until(&self, target: u64) -> bool {
        for seq in self.next_sequence..=target {
            if !self.messages.contains_key(&seq) {
                return false;
            }
        }
        true
    }

    /// Returns `Option<Vec<u8>>` where it's `Some(bytes)` if there is gapless
    /// ordered data in the buffer, and `None` if the buffer is empty or has
    /// gaps in the contained data.
    ///
    /// E.g. if the buffer contains messages with indexes 0, 1, 2, and 4, then
    /// a read will return the bytes of messages 0, 1, 2. Subsequent reads will
    /// return `None` until message 3 comes in, at which point 3, 4, and any
    /// further contiguous messages which have arrived will be returned.
    #[must_use]
    pub fn read(&mut self) -> Option<ReadContiguousData> {
        if !self.messages.contains_key(&self.next_sequence) {
            return None;
        }

        let mut contiguous_messages = Vec::new();
        let mut seq = self.next_sequence;

        while let Some(mut data) = self.messages.remove(&seq) {
            contiguous_messages.append(&mut data);
            seq += 1;
        }

        let high_water = seq;
        self.next_sequence = high_water;
        trace!("Next high water mark is: {high_water}");

        trace!(
            "Returning {} bytes from ordered message buffer",
            contiguous_messages.len()
        );
        Some(ReadContiguousData {
            data: contiguous_messages,
            last_sequence: self.next_sequence - 1,
        })
    }
}

impl Default for OrderedMessageBuffer {
    fn default() -> Self {
        OrderedMessageBuffer::new()
    }
}

#[cfg(test)]
mod test_chunking_and_reassembling {
    use super::*;

    #[test]
    fn trying_to_write_unreasonable_high_sequence() {
        let mut buffer = OrderedMessageBuffer::new();
        let first_message = vec![1, 2, 3, 4];
        let second_message = vec![5, 6, 7, 8];

        buffer.write(0, first_message).unwrap();
        buffer.write(1, second_message).unwrap();

        assert_eq!(
            Err(OrderedMessageError::MessageSequenceTooLarge {
                current: 0,
                received: 12345678
            }),
            buffer.write(12345678, b"foomp".to_vec())
        )
    }

    #[test]
    fn trying_to_overwrite_sequence() {
        let mut buffer = OrderedMessageBuffer::new();
        let message = vec![1, 2, 3, 4];

        buffer.write(0, message.clone()).unwrap();
        buffer.write(1, message.clone()).unwrap();
        buffer.write(2, message.clone()).unwrap();
        buffer.write(3, message.clone()).unwrap();

        for seq in 0..=3 {
            assert_eq!(
                Err(OrderedMessageError::AttemptedToOverwriteSequence { received: seq }),
                buffer.write(seq, message.clone())
            )
        }
    }

    #[test]
    fn writing_past_data() {
        let mut buffer = OrderedMessageBuffer::new();
        let message = vec![1, 2, 3, 4];

        buffer.write(0, message.clone()).unwrap();
        buffer.write(1, message.clone()).unwrap();
        buffer.write(2, message.clone()).unwrap();
        buffer.write(3, message.clone()).unwrap();
        let _ = buffer.read().unwrap();

        for seq in 0..=3 {
            assert_eq!(
                Err(OrderedMessageError::MessageAlreadyReconstructed {
                    current: 4,
                    received: seq
                }),
                buffer.write(seq, message.clone())
            )
        }
    }

    #[cfg(test)]
    mod reading_from_and_writing_to_the_buffer {
        use super::*;

        #[cfg(test)]
        mod when_full_ordered_sequence_exists {
            use super::*;

            #[test]
            fn read_returns_ordered_bytes_and_resets_buffer() {
                let mut buffer = OrderedMessageBuffer::new();

                let first_message = vec![1, 2, 3, 4];
                let second_message = vec![5, 6, 7, 8];

                buffer.write(0, first_message).unwrap();
                let first_read = buffer.read().unwrap().data;
                assert_eq!(vec![1, 2, 3, 4], first_read);

                buffer.write(1, second_message).unwrap();
                let second_read = buffer.read().unwrap().data;
                assert_eq!(vec![5, 6, 7, 8], second_read);

                assert_eq!(None, buffer.read()); // second read on fully ordered result set is empty
            }

            #[test]
            fn test_multiple_adds_stacks_up_bytes_in_the_buffer() {
                let mut buffer = OrderedMessageBuffer::new();

                let first_message = vec![1, 2, 3, 4];
                let second_message = vec![5, 6, 7, 8];

                buffer.write(0, first_message).unwrap();
                buffer.write(1, second_message).unwrap();
                let second_read = buffer.read();
                assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8], second_read.unwrap().data);
                assert_eq!(None, buffer.read()); // second read on fully ordered result set is empty
            }

            #[test]
            fn out_of_order_adds_results_in_ordered_byte_vector() {
                let mut buffer = OrderedMessageBuffer::new();

                let first_message = vec![1, 2, 3, 4];
                let second_message = vec![5, 6, 7, 8];

                buffer.write(1, second_message).unwrap();
                buffer.write(0, first_message).unwrap();
                let read = buffer.read().unwrap().data;
                assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8], read);
                assert_eq!(None, buffer.read()); // second read on fully ordered result set is empty
            }
        }

        mod when_there_are_gaps_in_the_sequence {
            use super::*;

            #[cfg(test)]
            fn setup() -> OrderedMessageBuffer {
                let mut buffer = OrderedMessageBuffer::new();

                let zero_message = vec![0, 0, 0, 0];
                let one_message = vec![1, 1, 1, 1];
                let three_message = vec![3, 3, 3, 3];

                buffer.write(0, zero_message).unwrap();
                buffer.write(1, one_message).unwrap();
                buffer.write(3, three_message).unwrap();
                buffer
            }
            #[test]
            fn everything_up_to_the_indexing_gap_is_returned() {
                let mut buffer = setup();
                let ordered_bytes = buffer.read().unwrap().data;
                assert_eq!([0, 0, 0, 0, 1, 1, 1, 1].to_vec(), ordered_bytes);

                // we shouldn't get any more from a second attempt if nothing is added
                assert_eq!(None, buffer.read());

                // let's add another message, leaving a gap in place at index 2
                let five_message = vec![5, 5, 5, 5];
                buffer.write(5, five_message).unwrap();
                assert_eq!(None, buffer.read());
            }

            #[test]
            fn filling_the_gap_allows_us_to_get_everything() {
                let mut buffer = setup();
                let _ = buffer.read(); // that burns the first two. We still have a gap before the 3s.

                let two_message = vec![2, 2, 2, 2];
                buffer.write(2, two_message).unwrap();

                let more_ordered_bytes = buffer.read().unwrap().data;
                assert_eq!([2, 2, 2, 2, 3, 3, 3, 3].to_vec(), more_ordered_bytes);

                // let's add another message
                let five_message = vec![5, 5, 5, 5];
                buffer.write(5, five_message).unwrap();

                assert_eq!(None, buffer.read());

                // let's fill in the gap of 4s now and read again
                let four_message = vec![4, 4, 4, 4];
                buffer.write(4, four_message).unwrap();

                assert_eq!(
                    [4, 4, 4, 4, 5, 5, 5, 5].to_vec(),
                    buffer.read().unwrap().data
                );

                // at this point we should again get back nothing if we try a read
                assert_eq!(None, buffer.read());
            }

            #[test]
            fn filling_the_gap_allows_us_to_get_everything_when_last_element_is_empty() {
                let mut buffer = OrderedMessageBuffer::new();
                let zero_message = vec![0, 0, 0, 0];
                let one_message = vec![2, 2, 2, 2];
                let two_message = vec![];

                buffer.write(0, zero_message).unwrap();
                assert!(buffer.read().is_some()); // burn the buffer

                buffer.write(2, two_message).unwrap();
                buffer.write(1, one_message).unwrap();
                assert!(buffer.read().is_some());
                assert_eq!(buffer.next_sequence, 3);
            }

            #[test]
            fn works_with_gaps_bigger_than_one() {
                let mut buffer = OrderedMessageBuffer::new();
                let zero_message = vec![0, 0, 0, 0];
                let one_message = vec![2, 2, 2, 2];
                let two_message = vec![2, 2, 2, 2];
                let three_message = vec![2, 2, 2, 2];
                let four_message = vec![2, 2, 2, 2];

                buffer.write(0, zero_message).unwrap();
                assert!(buffer.read().is_some());
                assert_eq!(buffer.next_sequence, 1);

                buffer.write(4, four_message).unwrap();
                assert!(buffer.read().is_none());
                assert_eq!(buffer.next_sequence, 1);

                buffer.write(3, three_message).unwrap();
                assert!(buffer.read().is_none());
                assert_eq!(buffer.next_sequence, 1);

                buffer.write(2, two_message).unwrap();
                assert!(buffer.read().is_none());
                assert_eq!(buffer.next_sequence, 1);

                buffer.write(1, one_message).unwrap();
                assert!(buffer.read().is_some());
                assert_eq!(buffer.next_sequence, 5)
            }
        }
    }
}
