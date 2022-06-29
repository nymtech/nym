use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub enum MessageError {
    NoData,
    IndexTooShort,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedMessage {
    pub data: Vec<u8>,
    pub index: u64,
}

impl OrderedMessage {
    /// Serializes an `OrderedMessage` into bytes.
    /// The output format is:
    /// | 8 bytes index | data... |
    pub fn into_bytes(self) -> Vec<u8> {
        self.index
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(self.data.into_iter())
            .collect()
    }

    /// Attempts to deserialize an `OrderedMessage` from bytes.
    pub fn try_from_bytes(data: Vec<u8>) -> Result<OrderedMessage, MessageError> {
        if data.is_empty() {
            return Err(MessageError::NoData);
        }

        if data.len() < 8 {
            return Err(MessageError::IndexTooShort);
        }
        let index = u64::from_be_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        Ok(OrderedMessage {
            data: data[8..].to_vec(),
            index,
        })
    }
}

/// Order messages by their index only, ignoring their data
impl PartialOrd for OrderedMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some((self.index).cmp(&(other.index)))
    }
}

#[cfg(test)]
mod ordered_message_to_bytes {
    use super::*;

    #[test]
    fn works() {
        let message = OrderedMessage {
            data: vec![123],
            index: 1,
        };
        let bytes = message.into_bytes();

        let expected = vec![0, 0, 0, 0, 0, 0, 0, 1, 123];
        assert_eq!(expected, bytes);
    }
}

#[cfg(test)]
mod ordered_message_from_bytes {
    use super::*;

    #[test]
    fn fails_when_there_is_no_data() {
        let result = OrderedMessage::try_from_bytes(Vec::new());
        assert_eq!(Err(MessageError::NoData), result);
    }

    #[test]
    fn fails_when_data_is_too_short() {
        let result = OrderedMessage::try_from_bytes(vec![1, 2, 3]);
        assert_eq!(Err(MessageError::IndexTooShort), result);
    }

    #[test]
    fn works_when_there_is_enough_to_make_a_sequence_number_but_no_message_data() {
        let expected = OrderedMessage {
            data: Vec::new(),
            index: 1,
        };
        let result = OrderedMessage::try_from_bytes(vec![0, 0, 0, 0, 0, 0, 0, 1]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn works_when_there_is_seq_number_and_data() {
        let expected = OrderedMessage {
            data: vec![255, 255, 255],
            index: 1,
        };
        let result =
            OrderedMessage::try_from_bytes(vec![0, 0, 0, 0, 0, 0, 0, 1, 255, 255, 255]).unwrap();
        assert_eq!(expected, result);
    }
}

#[test]
fn empty_message_does_not_affect_ordering() {
    let mut msg1 = OrderedMessage {
        data: vec![255, 255, 255],
        index: 1,
    };

    let mut msg2 = OrderedMessage {
        data: vec![],
        index: 2,
    };

    assert!(msg1 < msg2);

    msg1.index = 2;
    msg2.index = 1;

    assert!(msg1 > msg2);
}
