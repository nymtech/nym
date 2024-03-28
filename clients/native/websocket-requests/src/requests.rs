// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// all variable size data is always prefixed with u64 length
// tags are u8

use crate::error::{self, ErrorKind};
use crate::text::ClientRequestText;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::{AnonymousSenderTag, SENDER_TAG_SIZE};

use std::mem::size_of;

#[repr(u8)]
enum ClientRequestTag {
    /// Value tag representing [`Send`] variant of the [`ClientRequest`]
    Send = 0x00,

    /// Value tag representing [`SendAnonymous`] variant of the [`ClientRequest`]
    SendAnonymous = 0x01,

    /// Value tag representing [`Reply`] variant of the [`ClientRequest`]
    Reply = 0x02,

    /// Value tag representing [`SelfAddress`] variant of the [`ClientRequest`]
    SelfAddress = 0x03,

    /// Value tag representing [`ClosedConnection`] variant of the [`ClientRequest`]
    ClosedConnection = 0x04,

    /// Value tag representing [`GetLaneQueueLength`] variant of the [`ClientRequest`]
    GetLaneQueueLength = 0x05,
}

impl TryFrom<u8> for ClientRequestTag {
    type Error = error::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Send as u8) => Ok(Self::Send),
            _ if value == (Self::SendAnonymous as u8) => Ok(Self::SendAnonymous),
            _ if value == (Self::Reply as u8) => Ok(Self::Reply),
            _ if value == (Self::SelfAddress as u8) => Ok(Self::SelfAddress),
            _ if value == (Self::ClosedConnection as u8) => Ok(Self::ClosedConnection),
            _ if value == (Self::GetLaneQueueLength as u8) => Ok(Self::GetLaneQueueLength),
            n => Err(error::Error::new(
                ErrorKind::UnknownRequest,
                format!("{n} does not correspond to any valid request tag"),
            )),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub enum ClientRequest {
    /// The simplest message variant where no additional information is attached.
    /// You're simply sending your `data` to specified `recipient` without any tagging.
    ///
    /// Ends up with `NymMessage::Plain` variant
    Send {
        recipient: Recipient,
        message: Vec<u8>,
        connection_id: Option<u64>,
    },

    /// Create a message used for a duplex anonymous communication where the recipient
    /// will never learn of our true identity. This is achieved by carefully sending `reply_surbs`.
    ///
    /// Note that if reply_surbs is set to zero then
    /// this variant requires the client having sent some reply_surbs in the past
    /// (and thus the recipient also knowing our sender tag).
    ///
    /// Ends up with `NymMessage::Repliable` variant
    SendAnonymous {
        recipient: Recipient,
        message: Vec<u8>,
        reply_surbs: u32,
        connection_id: Option<u64>,
    },

    /// Attempt to use our internally received and stored `ReplySurb` to send the message back
    /// to specified recipient whilst not knowing its full identity (or even gateway).
    ///
    /// Ends up with `NymMessage::Reply` variant
    Reply {
        sender_tag: AnonymousSenderTag,
        message: Vec<u8>,
        connection_id: Option<u64>,
    },

    SelfAddress,

    ClosedConnection(u64),

    GetLaneQueueLength(u64),
}

// we could have been parsing it directly TryFrom<WsMessage>, but we want to retain
// information about whether it came from binary or text to send appropriate response back
impl ClientRequest {
    // SEND_REQUEST_TAG || recipient || conn_id || data_len || data
    fn serialize_send(recipient: Recipient, data: Vec<u8>, connection_id: Option<u64>) -> Vec<u8> {
        let data_len_bytes = (data.len() as u64).to_be_bytes();
        let conn_id_bytes = connection_id.unwrap_or(0).to_be_bytes();

        std::iter::once(ClientRequestTag::Send as u8)
            .chain(recipient.to_bytes()) // will not be length prefixed because the length is constant
            .chain(conn_id_bytes)
            .chain(data_len_bytes)
            .chain(data)
            .collect()
    }

    // SEND_REQUEST_TAG || recipient || conn_id || data_len || data
    fn deserialize_send(b: &[u8]) -> Result<Self, error::Error> {
        // we need to have at least 1 (tag) + Recipient::LEN + 2*sizeof<u64> bytes
        if b.len() < 1 + Recipient::LEN + 2 * size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'send'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::Send as u8);

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[1..1 + Recipient::LEN]);
        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(error::Error::new(
                    ErrorKind::MalformedRequest,
                    format!("malformed recipient: {err}"),
                ))
            }
        };

        let mut connection_id_bytes = [0u8; size_of::<u64>()];
        connection_id_bytes
            .copy_from_slice(&b[1 + Recipient::LEN..1 + Recipient::LEN + size_of::<u64>()]);
        let connection_id = u64::from_be_bytes(connection_id_bytes);
        let connection_id = if connection_id == 0 {
            None
        } else {
            Some(connection_id)
        };

        let data_len_bytes =
            &b[1 + Recipient::LEN + size_of::<u64>()..1 + Recipient::LEN + 2 * size_of::<u64>()];
        let data_len = u64::from_be_bytes(data_len_bytes.try_into().unwrap());
        let data = &b[1 + Recipient::LEN + 2 * size_of::<u64>()..];
        if data.len() as u64 != data_len {
            return Err(error::Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "data len has inconsistent length. specified: {} got: {}",
                    data_len,
                    data.len()
                ),
            ));
        }

        Ok(ClientRequest::Send {
            recipient,
            message: data.to_vec(),
            connection_id,
        })
    }

    // SEND_ANONYMOUS_REQUEST_TAG || reply_surbs || recipient || conn_id || data_len || data
    fn serialize_send_anonymous(
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        connection_id: Option<u64>,
    ) -> Vec<u8> {
        let data_len_bytes = (data.len() as u64).to_be_bytes();
        let conn_id_bytes = connection_id.unwrap_or(0).to_be_bytes();

        std::iter::once(ClientRequestTag::SendAnonymous as u8)
            .chain(reply_surbs.to_be_bytes())
            .chain(recipient.to_bytes()) // will not be length prefixed because the length is constant
            .chain(conn_id_bytes)
            .chain(data_len_bytes)
            .chain(data)
            .collect()
    }

    // SEND_ANONYMOUS_REQUEST_TAG || reply_surbs || recipient || data_len || data
    fn deserialize_send_anonymous(b: &[u8]) -> Result<Self, error::Error> {
        // we need to have at least 1 (tag) + sizeof<u32> (num surbs) + Recipient::LEN + 2 *sizeof<u64> bytes
        if b.len() < 1 + size_of::<u32>() + Recipient::LEN + 2 * size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'send_anonymous'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::SendAnonymous as u8);

        let reply_surbs = u32::from_be_bytes([b[1], b[2], b[3], b[4]]);

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[5..5 + Recipient::LEN]);
        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(error::Error::new(
                    ErrorKind::MalformedRequest,
                    format!("malformed recipient: {err}"),
                ))
            }
        };

        let mut connection_id_bytes = [0u8; size_of::<u64>()];
        connection_id_bytes
            .copy_from_slice(&b[5 + Recipient::LEN..5 + Recipient::LEN + size_of::<u64>()]);
        let connection_id = u64::from_be_bytes(connection_id_bytes);
        let connection_id = if connection_id == 0 {
            None
        } else {
            Some(connection_id)
        };

        let data_len_bytes =
            &b[5 + Recipient::LEN + size_of::<u64>()..5 + Recipient::LEN + 2 * size_of::<u64>()];
        let data_len = u64::from_be_bytes(data_len_bytes.try_into().unwrap());
        let data = &b[5 + Recipient::LEN + 2 * size_of::<u64>()..];
        if data.len() as u64 != data_len {
            return Err(error::Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "data len has inconsistent length. specified: {} got: {}",
                    data_len,
                    data.len()
                ),
            ));
        }

        Ok(ClientRequest::SendAnonymous {
            reply_surbs,
            recipient,
            message: data.to_vec(),
            connection_id,
        })
    }

    // REPLY_REQUEST_TAG || SENDER_TAG || conn_id || message_len || message
    fn serialize_reply(
        message: Vec<u8>,
        sender_tag: AnonymousSenderTag,
        connection_id: Option<u64>,
    ) -> Vec<u8> {
        let message_len_bytes = (message.len() as u64).to_be_bytes();
        let conn_id_bytes = connection_id.unwrap_or(0).to_be_bytes();

        std::iter::once(ClientRequestTag::Reply as u8)
            .chain(sender_tag.to_bytes())
            .chain(conn_id_bytes)
            .chain(message_len_bytes)
            .chain(message)
            .collect()
    }

    // REPLY_REQUEST_TAG || SENDER_TAG || conn_id || message_len || message
    fn deserialize_reply(b: &[u8]) -> Result<Self, error::Error> {
        if b.len() < 1 + SENDER_TAG_SIZE + 2 * size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'reply'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::Reply as u8);

        // the unwrap here is fine as we're definitely using exactly SENDER_TAG_SIZE bytes
        let sender_tag =
            AnonymousSenderTag::from_bytes(b[1..1 + SENDER_TAG_SIZE].try_into().unwrap());

        let mut connection_id_bytes = [0u8; size_of::<u64>()];
        connection_id_bytes
            .copy_from_slice(&b[1 + SENDER_TAG_SIZE..1 + SENDER_TAG_SIZE + size_of::<u64>()]);
        let connection_id = u64::from_be_bytes(connection_id_bytes);
        let connection_id = if connection_id == 0 {
            None
        } else {
            Some(connection_id)
        };

        let message_len = u64::from_be_bytes(
            b[1 + SENDER_TAG_SIZE + size_of::<u64>()..1 + SENDER_TAG_SIZE + 2 * size_of::<u64>()]
                .try_into()
                .unwrap(),
        );
        let message = &b[1 + SENDER_TAG_SIZE + 2 * size_of::<u64>()..];
        if message.len() as u64 != message_len {
            return Err(error::Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "message len has inconsistent length. specified: {} got: {}",
                    message_len,
                    message.len()
                ),
            ));
        }

        Ok(ClientRequest::Reply {
            message: message.to_vec(),
            sender_tag,
            connection_id,
        })
    }

    // SELF_ADDRESS_REQUEST_TAG
    fn serialize_self_address() -> Vec<u8> {
        vec![ClientRequestTag::SelfAddress as u8]
    }

    // SELF_ADDRESS_REQUEST_TAG
    fn deserialize_self_address(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::SelfAddress as u8);

        Ok(ClientRequest::SelfAddress)
    }

    // CLOSED_CONNECTION_REQUEST_TAG
    fn serialize_closed_connection(connection_id: u64) -> Vec<u8> {
        let conn_id_bytes = connection_id.to_be_bytes();
        std::iter::once(ClientRequestTag::ClosedConnection as u8)
            .chain(conn_id_bytes)
            .collect()
    }

    // CLOSED_CONNECTION_REQUEST_TAG
    fn deserialize_closed_connection(b: &[u8]) -> Result<Self, error::Error> {
        if b.len() != 1 + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::MalformedRequest,
                "The received closed connection has invalid length",
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::ClosedConnection as u8);

        let mut connection_id_bytes = [0u8; size_of::<u64>()];
        connection_id_bytes.copy_from_slice(&b[1..=size_of::<u64>()]);
        let connection_id = u64::from_be_bytes(connection_id_bytes);

        Ok(ClientRequest::ClosedConnection(connection_id))
    }

    // GET_LANE_QUEUE_LENGHT_TAG
    fn serialize_get_lane_queue_lengths(connection_id: u64) -> Vec<u8> {
        let conn_id_bytes = connection_id.to_be_bytes();
        std::iter::once(ClientRequestTag::GetLaneQueueLength as u8)
            .chain(conn_id_bytes)
            .collect()
    }

    // GET_LANE_QUEUE_LENGHT_TAG
    fn deserialize_get_lane_queue_length(b: &[u8]) -> Result<Self, error::Error> {
        if b.len() != 1 + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::MalformedRequest,
                "The received get lane queue lengths has invalid length",
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::GetLaneQueueLength as u8);

        let mut connection_id_bytes = [0u8; size_of::<u64>()];
        connection_id_bytes.copy_from_slice(&b[1..=size_of::<u64>()]);
        let connection_id = u64::from_be_bytes(connection_id_bytes);

        Ok(ClientRequest::GetLaneQueueLength(connection_id))
    }

    pub fn serialize(self) -> Vec<u8> {
        match self {
            ClientRequest::Send {
                recipient,
                message,
                connection_id,
            } => Self::serialize_send(recipient, message, connection_id),

            ClientRequest::SendAnonymous {
                recipient,
                message,
                reply_surbs,
                connection_id,
            } => Self::serialize_send_anonymous(recipient, message, reply_surbs, connection_id),

            ClientRequest::Reply {
                message,
                sender_tag,
                connection_id,
            } => Self::serialize_reply(message, sender_tag, connection_id),

            ClientRequest::SelfAddress => Self::serialize_self_address(),

            ClientRequest::ClosedConnection(id) => Self::serialize_closed_connection(id),

            ClientRequest::GetLaneQueueLength(id) => Self::serialize_get_lane_queue_lengths(id),
        }
    }

    pub fn deserialize(b: &[u8]) -> Result<Self, error::Error> {
        if b.is_empty() {
            // technically I'm not even sure this can ever be returned, because reading empty
            // request would imply closed socket, but let's include it for completion sake
            return Err(error::Error::new(
                ErrorKind::EmptyRequest,
                "no data provided".to_string(),
            ));
        }

        let request_tag = ClientRequestTag::try_from(b[0])?;

        // determine what kind of request that is and try to deserialize it
        match request_tag {
            ClientRequestTag::Send => Self::deserialize_send(b),
            ClientRequestTag::SendAnonymous => Self::deserialize_send_anonymous(b),
            ClientRequestTag::Reply => Self::deserialize_reply(b),
            ClientRequestTag::SelfAddress => Self::deserialize_self_address(b),
            ClientRequestTag::ClosedConnection => Self::deserialize_closed_connection(b),
            ClientRequestTag::GetLaneQueueLength => Self::deserialize_get_lane_queue_length(b),
        }
    }

    pub fn try_from_binary(raw_req: &[u8]) -> Result<Self, error::Error> {
        Self::deserialize(raw_req)
    }

    pub fn try_from_text(raw_req: String) -> Result<Self, error::Error> {
        // use the intermediate string structure and let serde do bunch of work for us
        let text_req = ClientRequestText::try_from(raw_req).map_err(|json_err| {
            error::Error::new(ErrorKind::MalformedRequest, json_err.to_string())
        })?;

        text_req.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // very basic tests to check for obvious errors like off by one
    #[test]
    fn send_request_serialization_works() {
        let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
        let recipient_string = recipient.to_string();

        let send_request = ClientRequest::Send {
            recipient,
            message: b"foomp".to_vec(),
            connection_id: Some(42),
        };

        let bytes = send_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Send {
                recipient,
                message,
                connection_id,
            } => {
                assert_eq!(recipient.to_string(), recipient_string);
                assert_eq!(message, b"foomp".to_vec());
                assert_eq!(connection_id, Some(42))
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn send_anonymous_request_serialization_works() {
        let original_recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();

        let send_anonymous_request = ClientRequest::SendAnonymous {
            recipient: original_recipient,
            message: b"foomp".to_vec(),
            reply_surbs: 666,
            connection_id: Some(42),
        };

        let bytes = send_anonymous_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::SendAnonymous {
                recipient,
                message,
                reply_surbs,
                connection_id,
            } => {
                assert_eq!(recipient, original_recipient);
                assert_eq!(message, b"foomp".to_vec());
                assert_eq!(connection_id, Some(42));
                assert_eq!(reply_surbs, 666)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn reply_request_serialization_works() {
        let reply_request = ClientRequest::Reply {
            sender_tag: [8u8; SENDER_TAG_SIZE].into(),
            message: b"foomp".to_vec(),
            connection_id: Some(42),
        };

        let bytes = reply_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Reply {
                sender_tag,
                message,
                connection_id,
            } => {
                assert_eq!(sender_tag, [8u8; SENDER_TAG_SIZE].into());
                assert_eq!(message, b"foomp".to_vec());
                assert_eq!(connection_id, Some(42));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn self_address_request_serialization_works() {
        let self_address_request = ClientRequest::SelfAddress;
        let bytes = self_address_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::SelfAddress => (),
            _ => unreachable!(),
        }
    }

    #[test]
    fn close_connection_request_serialization_works() {
        let close_connection_request = ClientRequest::ClosedConnection(42);
        let bytes = close_connection_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::ClosedConnection(id) => assert_eq!(id, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn get_lane_queue_length_request_serialization_works() {
        let close_connection_request = ClientRequest::GetLaneQueueLength(42);
        let bytes = close_connection_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::GetLaneQueueLength(id) => assert_eq!(id, 42),
            _ => unreachable!(),
        }
    }
}
