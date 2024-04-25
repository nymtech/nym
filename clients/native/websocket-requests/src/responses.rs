// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// all variable size data is always prefixed with u64 length
// tags are u8

use crate::error::{self, ErrorKind};
use crate::text::ServerResponseText;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::{AnonymousSenderTag, SENDER_TAG_SIZE};
use nym_sphinx::receiver::ReconstructedMessage;

use std::mem::size_of;

#[repr(u8)]
enum ServerResponseTag {
    /// Value tag representing [`Error`] variant of the [`ServerResponse`]
    Error = 0x00,

    /// Value tag representing [`Received`] variant of the [`ServerResponse`]
    Received = 0x01,

    /// Value tag representing [`SelfAddress`] variant of the [`ServerResponse`]
    SelfAddress = 0x02,

    /// Value tag representing [`LaneQueueLength`] variant of the [`ServerResponse`]
    LaneQueueLength = 0x03,
}

impl TryFrom<u8> for ServerResponseTag {
    type Error = error::Error;

    fn try_from(value: u8) -> Result<Self, error::Error> {
        match value {
            _ if value == (Self::Error as u8) => Ok(Self::Error),
            _ if value == (Self::Received as u8) => Ok(Self::Received),
            _ if value == (Self::SelfAddress as u8) => Ok(Self::SelfAddress),
            _ if value == (Self::LaneQueueLength as u8) => Ok(Self::LaneQueueLength),
            n => Err(error::Error::new(
                ErrorKind::UnknownResponse,
                format!("{n} does not correspond to any valid response tag"),
            )),
        }
    }
}

#[derive(Debug)]
pub enum ServerResponse {
    Received(ReconstructedMessage),
    SelfAddress(Box<Recipient>),
    LaneQueueLength { lane: u64, queue_length: usize },
    Error(error::Error),
}

impl ServerResponse {
    pub fn new_error<S: Into<String>>(message: S) -> Self {
        ServerResponse::Error(error::Error {
            kind: ErrorKind::Other,
            message: message.into(),
        })
    }

    // RECEIVED_RESPONSE_TAG || 1 | 0 indicating sender_tag || Option<sender_tag> || msg_len || msg
    fn serialize_received(reconstructed_message: ReconstructedMessage) -> Vec<u8> {
        let message_len_bytes = (reconstructed_message.message.len() as u64).to_be_bytes();

        if let Some(sender_tag) = reconstructed_message.sender_tag {
            std::iter::once(ServerResponseTag::Received as u8)
                .chain(std::iter::once(true as u8))
                .chain(sender_tag.to_bytes())
                .chain(message_len_bytes.iter().cloned())
                .chain(reconstructed_message.message)
                .collect()
        } else {
            std::iter::once(ServerResponseTag::Received as u8)
                .chain(std::iter::once(false as u8))
                .chain(message_len_bytes.iter().cloned())
                .chain(reconstructed_message.message)
                .collect()
        }
    }

    // RECEIVED_RESPONSE_TAG || 1 | 0 indicating sender_tag || Option<sender_tag> || msg_len || msg
    fn deserialize_received(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'

        // we must be able to read at the very least if it has a reply_surb and length of some field
        if b.len() < 2 + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'received'".to_string(),
            ));
        }
        debug_assert_eq!(b[0], ServerResponseTag::Received as u8);

        let has_sender_tag = match b[1] {
            0 => false,
            1 => true,
            n => {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    format!("invalid sender tag flag {n}"),
                ))
            }
        };

        let mut i = 2;
        let sender_tag = if has_sender_tag {
            if b[2..].len() < SENDER_TAG_SIZE {
                return Err(error::Error::new(
                    ErrorKind::TooShortResponse,
                    "not enough data provided to recover 'received'".to_string(),
                ));
            }
            i += SENDER_TAG_SIZE;
            Some(AnonymousSenderTag::from_bytes(
                b[2..2 + SENDER_TAG_SIZE].try_into().unwrap(),
            ))
        } else {
            None
        };

        if b[i..].len() < size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'received'".to_string(),
            ));
        }

        let message_len = u64::from_be_bytes(b[i..i + size_of::<u64>()].try_into().unwrap());
        let message = &b[i + size_of::<u64>()..];
        if message.len() as u64 != message_len {
            return Err(error::Error::new(
                ErrorKind::MalformedResponse,
                format!(
                    "message len has inconsistent length. specified: {} got: {}",
                    message_len,
                    message.len()
                ),
            ));
        }

        Ok(ServerResponse::Received(ReconstructedMessage {
            message: message.to_vec(),
            sender_tag,
        }))
    }

    // SELF_ADDRESS_RESPONSE_TAG || self_address
    fn serialize_self_address(address: Recipient) -> Vec<u8> {
        std::iter::once(ServerResponseTag::SelfAddress as u8)
            .chain(address.to_bytes())
            .collect()
    }

    // SELF_ADDRESS_RESPONSE_TAG || self_address
    fn deserialize_self_address(b: &[u8]) -> Result<Self, error::Error> {
        if b.len() != 1 + Recipient::LEN {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'self_address'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ServerResponseTag::SelfAddress as u8);

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[1..1 + Recipient::LEN]);

        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    format!("malformed Recipient: {err}"),
                ))
            }
        };

        Ok(ServerResponse::SelfAddress(Box::new(recipient)))
    }

    // LANE_QUEUE_LENGTH_RESPONSE_TAG || lane || queue_length
    fn serialize_lane_queue_length(lane: u64, queue_length: usize) -> Vec<u8> {
        std::iter::once(ServerResponseTag::LaneQueueLength as u8)
            .chain(lane.to_be_bytes().iter().cloned())
            .chain(queue_length.to_be_bytes().iter().cloned())
            .collect()
    }

    // LANE_QUEUE_LENGTH_RESPONSE_TAG || lane || queue_length
    fn deserialize_lane_queue_length(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ServerResponseTag::LaneQueueLength as u8);

        let mut lane_bytes = [0u8; size_of::<u64>()];
        lane_bytes.copy_from_slice(&b[1..=size_of::<u64>()]);
        let lane = u64::from_be_bytes(lane_bytes);

        let mut queue_length_bytes = [0u8; size_of::<usize>()];
        queue_length_bytes
            .copy_from_slice(&b[1 + size_of::<u64>()..1 + size_of::<u64>() + size_of::<usize>()]);
        let queue_length = usize::from_be_bytes(queue_length_bytes);

        Ok(ServerResponse::LaneQueueLength { lane, queue_length })
    }

    // ERROR_RESPONSE_TAG || err_code || msg_len || msg
    fn serialize_error(error: error::Error) -> Vec<u8> {
        let message_len_bytes = (error.message.len() as u64).to_be_bytes();
        std::iter::once(ServerResponseTag::Error as u8)
            .chain(std::iter::once(error.kind as u8))
            .chain(message_len_bytes)
            .chain(error.message.into_bytes())
            .collect()
    }

    // ERROR_RESPONSE_TAG || err_code || msg_len || msg
    fn deserialize_error(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ServerResponseTag::Error as u8);

        if b.len() < size_of::<u8>() + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'error'".to_string(),
            ));
        }

        let error_kind = ErrorKind::try_from(b[1])?;

        let message_len = u64::from_be_bytes(b[2..2 + size_of::<u64>()].try_into().unwrap());
        let message = &b[2 + size_of::<u64>()..];
        if message.len() as u64 != message_len {
            return Err(error::Error::new(
                ErrorKind::MalformedResponse,
                format!(
                    "message len has inconsistent length. specified: {} got: {}",
                    message_len,
                    message.len()
                ),
            ));
        }

        let err_message = match String::from_utf8(message.to_vec()) {
            Ok(msg) => msg,
            Err(err) => {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    format!("malformed error message: {err}"),
                ))
            }
        };

        Ok(ServerResponse::Error(error::Error::new(
            error_kind,
            err_message,
        )))
    }

    pub fn serialize(self) -> Vec<u8> {
        match self {
            ServerResponse::Received(reconstructed_message) => {
                Self::serialize_received(reconstructed_message)
            }
            ServerResponse::SelfAddress(address) => Self::serialize_self_address(*address),
            ServerResponse::LaneQueueLength { lane, queue_length } => {
                Self::serialize_lane_queue_length(lane, queue_length)
            }
            ServerResponse::Error(err) => Self::serialize_error(err),
        }
    }

    pub fn deserialize(b: &[u8]) -> Result<Self, error::Error> {
        if b.is_empty() {
            // technically I'm not even sure this can ever be returned, because reading empty
            // request would imply closed socket, but let's include it for completion sake
            return Err(error::Error::new(
                ErrorKind::EmptyResponse,
                "no data provided".to_string(),
            ));
        }

        if b.len() < size_of::<u8>() {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                format!(
                    "not enough data provided to recover response tag. Provided only {} bytes",
                    b.len()
                ),
            ));
        }

        let response_tag = ServerResponseTag::try_from(b[0])?;

        // determine what kind of response that is and try to deserialize it
        match response_tag {
            ServerResponseTag::Received => Self::deserialize_received(b),
            ServerResponseTag::SelfAddress => Self::deserialize_self_address(b),
            ServerResponseTag::LaneQueueLength => Self::deserialize_lane_queue_length(b),
            ServerResponseTag::Error => Self::deserialize_error(b),
        }
    }

    pub fn into_binary(self) -> Vec<u8> {
        self.serialize()
    }

    pub fn into_text(self) -> String {
        // use the intermediate string structure and let serde do bunch of work for us
        let text_resp = ServerResponseText::from(self);

        text_resp.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn received_response_serialization_works() {
        let received_with_sender_tag = ServerResponse::Received(ReconstructedMessage {
            message: b"foomp".to_vec(),
            sender_tag: Some([42u8; SENDER_TAG_SIZE].into()),
        });
        let bytes = received_with_sender_tag.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Received(reconstructed) => {
                assert_eq!(reconstructed.message, b"foomp".to_vec());
                assert_eq!(
                    reconstructed.sender_tag,
                    Some([42u8; SENDER_TAG_SIZE].into())
                )
            }
            _ => unreachable!(),
        }

        let received_without_sender_tag = ServerResponse::Received(ReconstructedMessage {
            message: b"foomp".to_vec(),
            sender_tag: None,
        });
        let bytes = received_without_sender_tag.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Received(reconstructed) => {
                assert_eq!(reconstructed.message, b"foomp".to_vec());
                assert!(reconstructed.sender_tag.is_none())
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn self_address_response_serialization_works() {
        let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
        let recipient_string = recipient.to_string();

        let self_address_response = ServerResponse::SelfAddress(Box::new(recipient));
        let bytes = self_address_response.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::SelfAddress(recipient) => {
                assert_eq!(recipient.to_string(), recipient_string)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn lane_queue_length_response_serialization_works() {
        let lane_queue_length_response = ServerResponse::LaneQueueLength {
            lane: 13,
            queue_length: 42,
        };
        let bytes = lane_queue_length_response.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::LaneQueueLength { lane, queue_length } => {
                assert_eq!(lane, 13);
                assert_eq!(queue_length, 42)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn error_response_serialization_works() {
        let dummy_error = error::Error::new(ErrorKind::UnknownRequest, "foomp message".to_string());
        let error_response = ServerResponse::Error(dummy_error.clone());
        let bytes = error_response.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Error(error) => assert_eq!(error, dummy_error),
            _ => unreachable!(),
        }
    }
}
