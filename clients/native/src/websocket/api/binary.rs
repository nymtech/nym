// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::error::{Error, ErrorKind};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySURB;
use nymsphinx::params::{MessageType, DEFAULT_NUM_MIX_HOPS};
use nymsphinx::receiver::ReconstructedMessage;
use std::convert::TryInto;
use std::mem::size_of;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

// all variable size data is always prefixed with u64 length
// tags are u8

/// Value tag representing [`Send`] variant of the [`ClientRequest`]
pub const SEND_REQUEST_TAG: u8 = 0x00;

/// Value tag representing [`Reply`] variant of the [`ClientRequest`]
pub const REPLY_REQUEST_TAG: u8 = 0x01;

/// Value tag representing [`SelfAddress`] variant of the [`ClientRequest`]
pub const SELF_ADDRESS_REQUEST_TAG: u8 = 0x02;

/// Value tag representing [`Error`] variant of the [`ServerResponse`]
pub const ERROR_RESPONSE_TAG: u8 = 0x00;

/// Value tag representing [`Received`] variant of the [`ServerResponse`]
pub const RECEIVED_RESPONSE_TAG: u8 = 0x01;

/// Value tag representing [`SelfAddress`] variant of the [`ServerResponse`]
pub const SELF_ADDRESS_RESPONSE_TAG: u8 = 0x02;

#[allow(non_snake_case)]
#[derive(Debug)]
pub enum ClientRequest {
    Send {
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
    },
    Reply {
        message: Vec<u8>,
        reply_surb: ReplySURB,
    },
    SelfAddress,
}

impl ClientRequest {
    // SEND_REQUEST_TAG || with_surb || recipient || data_len || data
    fn serialize_send(recipient: Recipient, data: Vec<u8>, with_reply_surb: bool) -> Vec<u8> {
        let data_len_bytes = (data.len() as u64).to_be_bytes();
        std::iter::once(SEND_REQUEST_TAG)
            .chain(std::iter::once(with_reply_surb as u8))
            .chain(recipient.to_bytes().iter().cloned()) // will not be length prefixed because the length is constant
            .chain(data_len_bytes.iter().cloned())
            .chain(data.into_iter())
            .collect()
    }

    // SEND_REQUEST_TAG || with_reply || recipient || data_len || data
    fn deserialize_send(b: &[u8]) -> Result<Self, Error> {
        // we need to have at least 1 (tag) + 1 (reply flag) + Recipient::LEN + sizeof<u64> bytes
        if b.len() < 2 + Recipient::LEN + size_of::<u64>() {
            return Err(Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'send'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], SEND_REQUEST_TAG);

        let with_reply_surb = match b[1] {
            0 => false,
            1 => true,
            n => {
                return Err(Error::new(
                    ErrorKind::MalformedRequest,
                    format!("invalid reply surb flag {}", n),
                ))
            }
        };

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[2..2 + Recipient::LEN]);
        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(Error::new(
                    ErrorKind::MalformedRequest,
                    format!("malformed recipient: {:?}", err),
                ))
            }
        };

        let data_len_bytes = &b[2 + Recipient::LEN..2 + Recipient::LEN + size_of::<u64>()];
        let data_len = u64::from_be_bytes(data_len_bytes.try_into().unwrap());
        let data = &b[2 + Recipient::LEN + size_of::<u64>()..];
        if data.len() as u64 != data_len {
            return Err(Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "data len has inconsistent length. specified: {} got: {}",
                    data_len,
                    data.len()
                ),
            ));
        }

        Ok(ClientRequest::Send {
            with_reply_surb,
            recipient,
            data: data.to_vec(),
        })
    }

    // REPLY_REQUEST_TAG || surb_len || surb || message_len || message
    fn serialize_reply(message: Vec<u8>, reply_surb: ReplySURB) -> Vec<u8> {
        let reply_surb_bytes = reply_surb.to_bytes();
        let surb_len_bytes = (reply_surb_bytes.len() as u64).to_be_bytes();
        let message_len_bytes = (message.len() as u64).to_be_bytes();

        std::iter::once(REPLY_REQUEST_TAG)
            .chain(surb_len_bytes.iter().cloned())
            .chain(reply_surb_bytes.into_iter())
            .chain(message_len_bytes.iter().cloned())
            .chain(message.into_iter())
            .collect()
    }

    // REPLY_REQUEST_TAG || surb_len || surb || message_len || message
    fn deserialize_reply(b: &[u8]) -> Result<Self, Error> {
        // we need to have at the very least 2 * sizeof<u64> bytes (in case, for some peculiar reason
        // message and reply surb were 0 len - the request would still be malformed, but would in theory
        // be parse'able)
        if b.len() < 1 + 2 * size_of::<u64>() {
            return Err(Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'reply'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], REPLY_REQUEST_TAG);

        let reply_surb_len =
            u64::from_be_bytes(b[1..1 + size_of::<u64>()].as_ref().try_into().unwrap());

        // make sure we won't go out of bounds here
        if reply_surb_len > (b.len() - 1 + 2 * size_of::<u64>()) as u64 {
            return Err(Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "not enough data to recover reply surb with specified length {}",
                    reply_surb_len
                ),
            ));
        }

        let surb_bound = 1 + size_of::<u64>() + reply_surb_len as usize;

        let reply_surb_bytes = &b[1 + size_of::<u64>()..surb_bound];
        let reply_surb = match ReplySURB::from_bytes(reply_surb_bytes) {
            Ok(reply_surb) => reply_surb,
            Err(err) => {
                return Err(Error::new(
                    ErrorKind::MalformedRequest,
                    format!("malformed reply surb: {:?}", err),
                ))
            }
        };

        let message_len = u64::from_be_bytes(
            b[surb_bound..surb_bound + size_of::<u64>()]
                .as_ref()
                .try_into()
                .unwrap(),
        );
        let message = &b[surb_bound + size_of::<u64>()..];
        if message.len() as u64 != message_len {
            return Err(Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "message len has inconsistent length. specified: {} got: {}",
                    message_len,
                    message.len()
                ),
            ));
        }
        // TODO: should this blow HERE, i.e. during deserialization that the data you're trying
        // to send via reply is too long?

        Ok(ClientRequest::Reply {
            reply_surb,
            message: message.to_vec(),
        })
    }

    // SELF_ADDRESS_REQUEST_TAG
    fn serialize_self_address() -> Vec<u8> {
        std::iter::once(SELF_ADDRESS_REQUEST_TAG).collect()
    }

    // SELF_ADDRESS_REQUEST_TAG
    fn deserialize_self_address(b: &[u8]) -> Result<Self, Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], SELF_ADDRESS_REQUEST_TAG);

        Ok(ClientRequest::SelfAddress)
    }

    pub fn serialize(self) -> Vec<u8> {
        match self {
            ClientRequest::Send {
                recipient,
                data,
                with_reply_surb,
            } => Self::serialize_send(recipient, data, with_reply_surb),

            ClientRequest::Reply {
                message,
                reply_surb,
            } => Self::serialize_reply(message, reply_surb),

            ClientRequest::SelfAddress => Self::serialize_self_address(),
        }
    }

    pub fn deserialize(b: &[u8]) -> Result<Self, Error> {
        if b.is_empty() {
            // technically I'm not even sure this can ever be returned, because reading empty
            // request would imply closed socket, but let's include it for completion sake
            return Err(Error::new(
                ErrorKind::EmptyRequest,
                "no data provided".to_string(),
            ));
        }

        if b.len() < size_of::<u8>() {
            return Err(Error::new(
                ErrorKind::TooShortRequest,
                format!(
                    "not enough data provided to recover request tag. Provided only {} bytes",
                    b.len()
                ),
            ));
        }
        let request_tag = b[0];

        // determine what kind of request that is and try to deserialize it
        match request_tag {
            SEND_REQUEST_TAG => Self::deserialize_send(b),
            REPLY_REQUEST_TAG => Self::deserialize_reply(b),
            SELF_ADDRESS_REQUEST_TAG => Self::deserialize_self_address(b),
            n => return Err(Error::new(ErrorKind::UnknownRequest, format!("type {}", n))),
        }
    }

    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!

    // TODO: I really think this should be done with something like protobuf / flatbuffers / Cap'n Proto,
    // especially if people using different languages had to use it
    // Another reason for some proper schema: messages pushed back to the client which will require
    // extra parsing to determine when the actual message starts and which parts are the reply surb

    // TODO2: perhaps do it the proper way and introduce an error type
    // TODO3: but if this 'stays' this way, the function could definitely use a clean up
    pub fn try_from_bytes(req: &[u8]) -> Option<Self> {
        if req.is_empty() {
            return None;
        }
        let with_reply_surb = match req[0] {
            n if n == MessageType::WithReplySURB as u8 => true,
            n if n == MessageType::WithoutReplySURB as u8 => false,
            n if n == MessageType::IsReply as u8 => {
                // TODO: this is extremely fragile as only works for the very specific network topology
                // and number of hops - another reason for some proper serialization library
                let surb_len = ReplySURB::serialized_len(DEFAULT_NUM_MIX_HOPS);

                if req.len() < surb_len + 1 {
                    return None;
                }

                // note the extra +1 (due to message prefix)
                let surb_bytes = &req[1..1 + surb_len];
                let reply_surb = ReplySURB::from_bytes(surb_bytes).ok()?;

                return Some(ClientRequest::Reply {
                    message: req[1 + surb_len..].to_vec(),
                    reply_surb,
                });
            }
            _ => return None, // no other option is valid in this context
        };

        if req.len() < Recipient::LEN + 1 {
            return None;
        }

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&req[1..Recipient::LEN + 1]);
        let recipient = Recipient::try_from_bytes(recipient_bytes).ok()?;

        Some(ClientRequest::Send {
            recipient,
            data: req[1 + Recipient::LEN..].to_vec(),
            with_reply_surb,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            // (MessageType::WithReplySURB OR MessageType::WithoutReplySURB) || RECIPIENT || MESSAGE
            ClientRequest::Send {
                recipient,
                data,
                with_reply_surb,
            } => std::iter::once(if with_reply_surb {
                MessageType::WithReplySURB as u8
            } else {
                MessageType::WithoutReplySURB as u8
            })
            .chain(recipient.to_bytes().iter().cloned())
            .chain(data.into_iter())
            .collect(),

            // MessageType::IsReply || REPLY_SURB || MESSAGE
            // TODO: this is fragile as reply_SURB length CAN BE variable. however temporarily
            // we are making 'unsafe' assumption that it will be constant
            ClientRequest::Reply {
                message,
                reply_surb,
            } => std::iter::once(MessageType::IsReply as u8)
                .chain(reply_surb.to_bytes().into_iter())
                .chain(message.into_iter())
                .collect(),

            ClientRequest::SelfAddress => todo!(),
        }
    }
}

impl Into<WsMessage> for ClientRequest {
    fn into(self) -> WsMessage {
        WsMessage::Binary(self.into_bytes())
    }
}

#[derive(Debug)]
pub enum ServerResponse {
    Received(ReconstructedMessage),
    SelfAddress(Recipient),
    Error(super::error::Error),
}

impl ServerResponse {
    // RECEIVED_RESPONSE_TAG || with_reply || (surb_len || surb) || msg_len || msg
    fn serialize_received(reconstructed_message: ReconstructedMessage) -> Vec<u8> {
        let message_len_bytes = (reconstructed_message.message.len() as u64).to_be_bytes();

        if let Some(reply_surb) = reconstructed_message.reply_SURB {
            let reply_surb_bytes = reply_surb.to_bytes();
            let surb_len_bytes = (reply_surb_bytes.len() as u64).to_be_bytes();

            // with_reply || surb_len || surb || msg_len || msg
            std::iter::once(RECEIVED_RESPONSE_TAG)
                .chain(std::iter::once(true as u8))
                .chain(surb_len_bytes.iter().cloned())
                .chain(reply_surb_bytes.iter().cloned())
                .chain(message_len_bytes.iter().cloned())
                .chain(reconstructed_message.message.into_iter())
                .collect()
        } else {
            // without_reply || msg_len || msg
            std::iter::once(RECEIVED_RESPONSE_TAG)
                .chain(std::iter::once(false as u8))
                .chain(message_len_bytes.iter().cloned())
                .chain(reconstructed_message.message.into_iter())
                .collect()
        }
    }

    // RECEIVED_RESPONSE_TAG || with_reply || (surb_len || surb) || msg_len || msg
    fn deserialize_received(b: &[u8]) -> Result<Self, Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], RECEIVED_RESPONSE_TAG);

        // we must be able to read at the very least if it has a reply_surb and length of some field
        if b.len() < 2 + size_of::<u64>() {
            return Err(Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'received'".to_string(),
            ));
        }

        let with_reply_surb = match b[1] {
            0 => false,
            1 => true,
            n => {
                return Err(Error::new(
                    ErrorKind::MalformedResponse,
                    format!("invalid reply flag {}", n),
                ))
            }
        };

        if with_reply_surb {
            let reply_surb_len =
                u64::from_be_bytes(b[2..2 + size_of::<u64>()].as_ref().try_into().unwrap());

            // make sure we won't go out of bounds here
            if reply_surb_len > (b.len() - 2 + 2 * size_of::<u64>()) as u64 {
                return Err(Error::new(
                    ErrorKind::MalformedResponse,
                    "not enough bytes to read reply_surb bytes!".to_string(),
                ));
            }

            let surb_bound = 2 + size_of::<u64>() + reply_surb_len as usize;

            let reply_surb_bytes = &b[2 + size_of::<u64>()..surb_bound];
            let reply_surb = match ReplySURB::from_bytes(reply_surb_bytes) {
                Ok(reply_surb) => reply_surb,
                Err(err) => {
                    return Err(Error::new(
                        ErrorKind::MalformedResponse,
                        format!("malformed reply SURB: {:?}", err),
                    ))
                }
            };

            let message_len = u64::from_be_bytes(
                b[surb_bound..surb_bound + size_of::<u64>()]
                    .as_ref()
                    .try_into()
                    .unwrap(),
            );
            let message = &b[surb_bound + size_of::<u64>()..];
            if message.len() as u64 != message_len {
                return Err(Error::new(
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
                reply_SURB: Some(reply_surb),
            }))
        } else {
            let message_len =
                u64::from_be_bytes(b[2..2 + size_of::<u64>()].as_ref().try_into().unwrap());
            let message = &b[2 + size_of::<u64>()..];
            if message.len() as u64 != message_len {
                return Err(Error::new(
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
                reply_SURB: None,
            }))
        }
    }

    // SELF_ADDRESS_RESPONSE_TAG || self_address
    fn serialize_self_address(address: Recipient) -> Vec<u8> {
        std::iter::once(SELF_ADDRESS_RESPONSE_TAG)
            .chain(address.to_bytes().iter().cloned())
            .collect()
    }

    // SELF_ADDRESS_RESPONSE_TAG || self_address
    fn deserialize_self_address(b: &[u8]) -> Result<Self, Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], SELF_ADDRESS_RESPONSE_TAG);

        if b.len() != 1 + Recipient::LEN {
            return Err(Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'self_address'".to_string(),
            ));
        }

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[1..1 + Recipient::LEN]);

        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(Error::new(
                    ErrorKind::MalformedResponse,
                    format!("malformed Recipient: {:?}", err),
                ))
            }
        };

        Ok(ServerResponse::SelfAddress(recipient))
    }

    // ERROR_RESPONSE_TAG || err_code || msg_len || msg
    fn serialize_error(error: Error) -> Vec<u8> {
        let message_len_bytes = (error.message.len() as u64).to_be_bytes();
        std::iter::once(ERROR_RESPONSE_TAG)
            .chain(std::iter::once(error.kind as u8))
            .chain(message_len_bytes.iter().cloned())
            .chain(error.message.into_bytes().into_iter())
            .collect()
    }

    // ERROR_RESPONSE_TAG || err_code || msg_len || msg
    fn deserialize_error(b: &[u8]) -> Result<Self, Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ERROR_RESPONSE_TAG);

        if b.len() < size_of::<u8>() + size_of::<u64>() {
            return Err(Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'error'".to_string(),
            ));
        }

        let error_kind = match b[1] {
            _ if b[1] == (ErrorKind::EmptyRequest as u8) => ErrorKind::EmptyRequest,
            _ if b[1] == (ErrorKind::TooShortRequest as u8) => ErrorKind::TooShortRequest,
            _ if b[1] == (ErrorKind::UnknownRequest as u8) => ErrorKind::UnknownRequest,
            _ if b[1] == (ErrorKind::MalformedRequest as u8) => ErrorKind::MalformedRequest,

            _ if b[1] == (ErrorKind::EmptyResponse as u8) => ErrorKind::EmptyResponse,
            _ if b[1] == (ErrorKind::TooShortResponse as u8) => ErrorKind::TooShortResponse,
            _ if b[1] == (ErrorKind::UnknownResponse as u8) => ErrorKind::UnknownResponse,
            _ if b[1] == (ErrorKind::MalformedResponse as u8) => ErrorKind::MalformedResponse,

            n => {
                return Err(Error::new(
                    ErrorKind::MalformedResponse,
                    format!("invalid error code {}", n),
                ))
            }
        };

        let message_len =
            u64::from_be_bytes(b[2..2 + size_of::<u64>()].as_ref().try_into().unwrap());
        let message = &b[2 + size_of::<u64>()..];
        if message.len() as u64 != message_len {
            return Err(Error::new(
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
                return Err(Error::new(
                    ErrorKind::MalformedResponse,
                    format!("malformed error message: {:?}", err),
                ))
            }
        };

        Ok(ServerResponse::Error(Error::new(error_kind, err_message)))
    }

    pub fn serialize(self) -> Vec<u8> {
        match self {
            ServerResponse::Received(reconstructed_message) => {
                Self::serialize_received(reconstructed_message)
            }
            ServerResponse::SelfAddress(address) => Self::serialize_self_address(address),
            ServerResponse::Error(err) => Self::serialize_error(err),
        }
    }

    pub fn deserialize(b: &[u8]) -> Result<Self, Error> {
        if b.is_empty() {
            // technically I'm not even sure this can ever be returned, because reading empty
            // request would imply closed socket, but let's include it for completion sake
            return Err(Error::new(
                ErrorKind::EmptyResponse,
                "no data provided".to_string(),
            ));
        }

        if b.len() < size_of::<u8>() {
            return Err(Error::new(
                ErrorKind::TooShortResponse,
                format!(
                    "not enough data provided to recover response tag. Provided only {} bytes",
                    b.len()
                ),
            ));
        }

        let response_tag = b[0];

        // determine what kind of response that is and try to deserialize it
        match response_tag {
            RECEIVED_RESPONSE_TAG => Self::deserialize_received(b),
            SELF_ADDRESS_RESPONSE_TAG => Self::deserialize_self_address(b),
            ERROR_RESPONSE_TAG => Self::deserialize_error(b),
            n => {
                return Err(Error::new(
                    ErrorKind::UnknownResponse,
                    format!("type {}", n),
                ))
            }
        }
    }

    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!
    // OLD CODE THAT'S STILL IN USE!!

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            // this happens to work because right now there's only a single possible binary response
            ServerResponse::Received(reconstructed_message) => reconstructed_message.into_bytes(),
            _ => todo!(),
        }
    }

    // TODO: dont be lazy and define error type and change it into Result<Self, Error>
    pub fn try_from_bytes(b: &[u8]) -> Option<Self> {
        // this happens to work because right now there's only a single possible binary response
        Some(ServerResponse::Received(
            ReconstructedMessage::try_from_bytes(b).ok()?,
        ))
    }
}

impl Into<WsMessage> for ServerResponse {
    fn into(self) -> WsMessage {
        WsMessage::Binary(self.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use bincode::{DefaultOptions, Options};
    use crypto::asymmetric::{encryption, identity};

    // very basic tests to check for obvious errors like off by one
    #[test]
    fn send_request_serialization_works() {
        let client_id_pair = identity::KeyPair::new();
        let client_enc_pair = encryption::KeyPair::new();
        let gateway_id_pair = identity::KeyPair::new();

        let recipient = Recipient::new(
            *client_id_pair.public_key(),
            client_enc_pair.public_key().clone(),
            *gateway_id_pair.public_key(),
        );
        let recipient_string = recipient.to_string();

        let send_request_no_surb = ClientRequest::Send {
            recipient: recipient.clone(),
            data: b"foomp".to_vec(),
            with_reply_surb: false,
        };

        let bytes = send_request_no_surb.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Send {
                recipient,
                data,
                with_reply_surb,
            } => {
                assert_eq!(recipient.to_string(), recipient_string);
                assert_eq!(data, b"foomp".to_vec());
                assert!(!with_reply_surb)
            }
            _ => unreachable!(),
        }

        let send_request_surb = ClientRequest::Send {
            recipient,
            data: b"foomp".to_vec(),
            with_reply_surb: true,
        };

        let bytes = send_request_surb.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Send {
                recipient,
                data,
                with_reply_surb,
            } => {
                assert_eq!(recipient.to_string(), recipient_string);
                assert_eq!(data, b"foomp".to_vec());
                assert!(with_reply_surb)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn reply_request_serialization_works() {
        let reply_surb_string = "CjfVbHbfAjbC3W1BvNHGXmM8KNAnDNYGaHMLqVDxRYeo352csAihstup9bvqXam4dTWgfHak6KYwL9STaxWJ47E8XFZbSEvs7hEsfCkxr6K9WJuSBPK84GDDEvad8ZAuMCoaXsAd5S2Lj9a5eYyzG4SL1jHzhSMni55LyJwumxo1ZTGZNXggxw1RREosvyzNrW9Rsi3owyPqLCwXpiei2tHZty8w8midVvg8vDa7ZEJD842CLv8D4ohynSG7gDpqTrhkRaqYAuz7dzqNbMXLJRM7v823Jn16fA1L7YQxmcaUdUigyRSgTdb4i9ebiLGSyJ1iDe6Acz613PQZh6Ua3bZ2zVKq3dSycpDm9ngarRK4zJrAaUxRkdih8YzW3BY4nL9eqkfKA4N1TWCLaRU7zpSaf8yMEwrAZReU3d5zLV8c5KBfa2w8R5anhQeBojduZEGEad8kkHuKU52Zg93FeWHvH1qgZaEJMHH4nN7gKXz9mvWDhYwyF4vt3Uy2NhCHC3N5pL1gMme27YcoPcTEia1fxKZtnt6rtEozzTrAgCJGswigkFbkafiV5QaJwLKTUxtzhkZ57eEuLPte9UvJHzhhXUQ2CV7R2BUkJjYZy3Zsx6YYvdYWiAFFkWUwNEGA4QpShUHciBfsQVHQ7pN41YcyYUhbywQDFnTVgEmdUZ1XCBi3gyK5U3tDQmFzP1u9m3mWrUA8qB9mRDE7ptNDm5c3c1458L6uXLUth7sdMaa1Was5LCmCdmNDtvNpCDAEt1in6q6mrZFR85aCSU9b1baNGwZoCqPpPvydkVe63gXWoi8ebvdyxARrqACFrSB3ZdY3uJBw8CTMNkKK6MvcefMkSVVsbLd36TQAtYSCqrpiMc5dQuKcEu5QfciwvWYXYx8WFNAgKwP2mv49KCTvfozNDUCbjzDwSx92Zv5zjG8HbFpB13bY9UZGeyTPvv7gGxCzjGjJGbW6FRAheRQaaje5fUgCNM95Tv7wBmAMRHHFgWafeK1sdFH7dtCX9u898HucGTaboSKLsVh8J78gbbkHErwjMh7y9YRkceq5TTYS5da4kHnyNKYWSbxgZrmFg44XGKoeYcqoHB3XTZrdsf7F5fFeNwnihkmADvhAcaxXUmVqq4rQFZH84a1iC3WBWXYcqiZH2L7ujGWV7mMDT4HBEerDYjc8rNY4xGTPfivCrBCJW1i14aqW8xRdsdgTM88eTksvC3WPJLJ7iMzfKXeL7fMW1Ek6QGyQtLBW98vEESpdcDg6DeZ5rMz6VqjTGGqcCaFGfHoqtfxMDaBAEsyQ8h7XDX6dg1wq9wH6j4Tw7Tj1MEv1b8uj5NJkozZdzVdYA2QyE2Dp8vuurQG6uVdTDNww2d88RBQ8sVgjxN8gR45y4woJLhFAaNTAtrY6wDTxyXST13ni6oyqdYxjFVk9Am4v3DzH7Y2K8iRVSHfTk4FRbPULyaeK6wt2anvMJH1XdvVRgc14h67MnBxMgMD1UFk8AErN7CDj26fppe3c5G6KozJe4cSqQUGbBjVzBnrHCruqrfZBn5hNZHTV37bQiomqhRQXohxhuKEnNrGbAe1xNvJr9X";
        let reply_surb = ReplySURB::from_base58_string(reply_surb_string).unwrap();
        let reply_request = ClientRequest::Reply {
            message: b"foomp".to_vec(),
            reply_surb,
        };

        let bytes = reply_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Reply {
                reply_surb,
                message,
            } => {
                assert_eq!(reply_surb.to_base58_string(), reply_surb_string);
                assert_eq!(message, b"foomp".to_vec());
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
    fn received_response_serialization_works() {
        let reply_surb_string = "CjfVbHbfAjbC3W1BvNHGXmM8KNAnDNYGaHMLqVDxRYeo352csAihstup9bvqXam4dTWgfHak6KYwL9STaxWJ47E8XFZbSEvs7hEsfCkxr6K9WJuSBPK84GDDEvad8ZAuMCoaXsAd5S2Lj9a5eYyzG4SL1jHzhSMni55LyJwumxo1ZTGZNXggxw1RREosvyzNrW9Rsi3owyPqLCwXpiei2tHZty8w8midVvg8vDa7ZEJD842CLv8D4ohynSG7gDpqTrhkRaqYAuz7dzqNbMXLJRM7v823Jn16fA1L7YQxmcaUdUigyRSgTdb4i9ebiLGSyJ1iDe6Acz613PQZh6Ua3bZ2zVKq3dSycpDm9ngarRK4zJrAaUxRkdih8YzW3BY4nL9eqkfKA4N1TWCLaRU7zpSaf8yMEwrAZReU3d5zLV8c5KBfa2w8R5anhQeBojduZEGEad8kkHuKU52Zg93FeWHvH1qgZaEJMHH4nN7gKXz9mvWDhYwyF4vt3Uy2NhCHC3N5pL1gMme27YcoPcTEia1fxKZtnt6rtEozzTrAgCJGswigkFbkafiV5QaJwLKTUxtzhkZ57eEuLPte9UvJHzhhXUQ2CV7R2BUkJjYZy3Zsx6YYvdYWiAFFkWUwNEGA4QpShUHciBfsQVHQ7pN41YcyYUhbywQDFnTVgEmdUZ1XCBi3gyK5U3tDQmFzP1u9m3mWrUA8qB9mRDE7ptNDm5c3c1458L6uXLUth7sdMaa1Was5LCmCdmNDtvNpCDAEt1in6q6mrZFR85aCSU9b1baNGwZoCqPpPvydkVe63gXWoi8ebvdyxARrqACFrSB3ZdY3uJBw8CTMNkKK6MvcefMkSVVsbLd36TQAtYSCqrpiMc5dQuKcEu5QfciwvWYXYx8WFNAgKwP2mv49KCTvfozNDUCbjzDwSx92Zv5zjG8HbFpB13bY9UZGeyTPvv7gGxCzjGjJGbW6FRAheRQaaje5fUgCNM95Tv7wBmAMRHHFgWafeK1sdFH7dtCX9u898HucGTaboSKLsVh8J78gbbkHErwjMh7y9YRkceq5TTYS5da4kHnyNKYWSbxgZrmFg44XGKoeYcqoHB3XTZrdsf7F5fFeNwnihkmADvhAcaxXUmVqq4rQFZH84a1iC3WBWXYcqiZH2L7ujGWV7mMDT4HBEerDYjc8rNY4xGTPfivCrBCJW1i14aqW8xRdsdgTM88eTksvC3WPJLJ7iMzfKXeL7fMW1Ek6QGyQtLBW98vEESpdcDg6DeZ5rMz6VqjTGGqcCaFGfHoqtfxMDaBAEsyQ8h7XDX6dg1wq9wH6j4Tw7Tj1MEv1b8uj5NJkozZdzVdYA2QyE2Dp8vuurQG6uVdTDNww2d88RBQ8sVgjxN8gR45y4woJLhFAaNTAtrY6wDTxyXST13ni6oyqdYxjFVk9Am4v3DzH7Y2K8iRVSHfTk4FRbPULyaeK6wt2anvMJH1XdvVRgc14h67MnBxMgMD1UFk8AErN7CDj26fppe3c5G6KozJe4cSqQUGbBjVzBnrHCruqrfZBn5hNZHTV37bQiomqhRQXohxhuKEnNrGbAe1xNvJr9X";

        let received_with_surb = ServerResponse::Received(ReconstructedMessage {
            message: b"foomp".to_vec(),
            reply_SURB: Some(ReplySURB::from_base58_string(reply_surb_string).unwrap()),
        });
        let bytes = received_with_surb.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Received(reconstructed) => {
                assert_eq!(reconstructed.message, b"foomp".to_vec());
                assert_eq!(
                    reconstructed.reply_SURB.unwrap().to_base58_string(),
                    reply_surb_string
                )
            }
            _ => unreachable!(),
        }

        let received_without_surb = ServerResponse::Received(ReconstructedMessage {
            message: b"foomp".to_vec(),
            reply_SURB: None,
        });
        let bytes = received_without_surb.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Received(reconstructed) => {
                assert_eq!(reconstructed.message, b"foomp".to_vec());
                assert!(reconstructed.reply_SURB.is_none())
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn self_address_response_serialization_works() {
        let client_id_pair = identity::KeyPair::new();
        let client_enc_pair = encryption::KeyPair::new();
        let gateway_id_pair = identity::KeyPair::new();

        let recipient = Recipient::new(
            *client_id_pair.public_key(),
            client_enc_pair.public_key().clone(),
            *gateway_id_pair.public_key(),
        );
        let recipient_string = recipient.to_string();

        let self_address_response = ServerResponse::SelfAddress(recipient);
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
    fn error_response_serialization_works() {
        let dummy_error = Error::new(ErrorKind::UnknownRequest, "foomp message".to_string());
        let error_response = ServerResponse::Error(dummy_error.clone());
        let bytes = error_response.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Error(error) => assert_eq!(error, dummy_error),
            _ => unreachable!(),
        }
    }
}
