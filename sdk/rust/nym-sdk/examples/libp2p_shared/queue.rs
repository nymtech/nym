use log::{debug, warn};
use std::collections::BTreeSet;

use super::message::TransportMessage;

/// MessageQueue is a queue of messages, ordered by nonce, that we've
/// received but are not yet able to process because we're waiting for
/// a message with the next expected nonce first.
/// This is required because Nym does not guarantee any sort of message
/// ordering, only delivery.
/// TODO: is there a DOS vector here where a malicious peer sends us
/// messages only with nonce higher than the next expected nonce?
pub(crate) struct MessageQueue {
    /// nonce of the next message we expect to receive on the
    /// connection.
    /// any messages with a nonce greater than this are pushed into
    /// the queue.
    /// if we get a message with a nonce equal to this, then we
    /// immediately handle it in the transport and increment the nonce.
    next_expected_nonce: u64,

    /// the actual queue of messages, ordered by nonce.
    /// the head of the queue's nonce is always greater
    /// than the next expected nonce.
    queue: BTreeSet<TransportMessage>,
}

impl MessageQueue {
    pub(crate) fn new() -> Self {
        MessageQueue {
            next_expected_nonce: 0,
            queue: BTreeSet::new(),
        }
    }

    pub(crate) fn print_nonces(&self) {
        let nonces = self.queue.iter().map(|msg| msg.nonce).collect::<Vec<_>>();
        debug!("MessageQueue: {:?}", nonces);
    }

    /// sets the next expected nonce to 1, indicating that we've received
    /// a ConnectionRequest or ConnectionResponse.
    pub(crate) fn set_connection_message_received(&mut self) {
        if self.next_expected_nonce != 0 {
            panic!("connection message received twice");
        }

        self.next_expected_nonce = self.next_expected_nonce.wrapping_add(1);
    }

    /// tries to push a message into the queue.
    /// if the message has the next expected nonce, then the message is returned,
    /// and should be processed by the caller.
    /// in that case, the internal queue's next expected nonce is incremented.
    pub(crate) fn try_push(&mut self, msg: TransportMessage) -> Option<TransportMessage> {
        if msg.nonce == self.next_expected_nonce {
            self.next_expected_nonce = self.next_expected_nonce.wrapping_add(1);
            Some(msg)
        } else {
            if msg.nonce < self.next_expected_nonce {
                // this shouldn't happen normally, only if the other node
                // is not following the protocol
                warn!("received a message with a nonce that is too low");
                return None;
            }

            if !self.queue.insert(msg) {
                // this shouldn't happen normally, only if the other node
                // is not following the protocol
                warn!("received a message with a duplicate nonce");
                return None;
            }

            None
        }
    }

    pub(crate) fn pop(&mut self) -> Option<TransportMessage> {
        let head = self.queue.first()?;

        if head.nonce == self.next_expected_nonce {
            self.next_expected_nonce = self.next_expected_nonce.wrapping_add(1);
            Some(self.queue.pop_first().unwrap())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::message::{ConnectionId, SubstreamId, SubstreamMessage};

    use super::*;

    impl TransportMessage {
        fn new(nonce: u64, message: SubstreamMessage, id: ConnectionId) -> Self {
            TransportMessage { nonce, message, id }
        }
    }

    #[test]
    fn test_message_queue() {
        let mut queue = MessageQueue::new();

        let test_substream_message =
            SubstreamMessage::new_with_data(SubstreamId::generate(), vec![1, 2, 3]);
        let connection_id = ConnectionId::generate();

        let msg1 = TransportMessage::new(1, test_substream_message.clone(), connection_id.clone());
        let msg2 = TransportMessage::new(2, test_substream_message.clone(), connection_id.clone());
        let msg3 = TransportMessage::new(3, test_substream_message.clone(), connection_id.clone());

        assert_eq!(queue.try_push(msg1.clone()), None);
        assert_eq!(queue.try_push(msg3.clone()), None);
        assert_eq!(queue.try_push(msg2.clone()), None);

        assert_eq!(queue.pop(), None);

        // set expected nonce to 1
        queue.set_connection_message_received();
        assert_eq!(queue.pop(), Some(msg1));

        let msg4 = TransportMessage::new(4, test_substream_message.clone(), connection_id.clone());
        assert_eq!(queue.try_push(msg4.clone()), None);

        assert_eq!(queue.pop(), Some(msg2));
        assert_eq!(queue.pop(), Some(msg3));
        assert_eq!(queue.pop(), Some(msg4));
        assert_eq!(queue.pop(), None);
        assert_eq!(queue.next_expected_nonce, 5);

        // should just return the message and increment nonce when message nonce = next expected nonce
        let msg5 = TransportMessage::new(5, test_substream_message, connection_id);
        assert_eq!(queue.try_push(msg5.clone()), Some(msg5));
        assert_eq!(queue.next_expected_nonce, 6);
    }
}
