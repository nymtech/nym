/// Assigns sequence numbers to outbound byte vectors. These messages can then
/// be reassembled into an ordered sequence by the `OrderedMessageSender`.
#[derive(Debug)]
pub struct OrderedMessageSender {
    next_sequence: u64,
}

impl OrderedMessageSender {
    pub fn new() -> OrderedMessageSender {
        OrderedMessageSender { next_sequence: 0 }
    }

    pub fn sequence(&mut self) -> u64 {
        let next = self.next_sequence;
        self.next_sequence += 1;
        next
    }

    // /// Turns raw bytes into an OrderedMessage containing the original bytes
    // /// and a sequence number;
    // pub fn wrap_message(&mut self, input: Vec<u8>) -> OrderedMessage {
    //     let message = OrderedMessage {
    //         data: input.to_vec(),
    //         index: self.next_sequence,
    //     };
    //     self.next_sequence += 1;
    //     message
    // }
}

impl Default for OrderedMessageSender {
    fn default() -> Self {
        OrderedMessageSender::new()
    }
}

#[cfg(test)]
mod ordered_message_sender {
    use super::*;

    mod when_input_bytes_are_empty {}

    #[cfg(test)]
    mod sequence_index_numbers {
        use super::*;

        #[test]
        fn increase_as_messages_are_sent() {
            let mut sender = OrderedMessageSender::new();

            let first_message = sender.sequence();

            assert_eq!(first_message, 0);

            let second_message = sender.sequence();
            assert_eq!(second_message, 1);
        }
    }
}
