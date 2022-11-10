// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// all variable size data is always prefixed with u64 length
// tags are u8

use crate::error::{self, ErrorKind};
use crate::text::ClientRequestText;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::{AnonymousSenderTag, SENDER_TAG_SIZE};
use nymsphinx::anonymous_replies::ReplySurb;
use std::convert::{TryFrom, TryInto};
use std::mem::size_of;

#[repr(u8)]
enum ClientRequestTag {
    /// Value tag representing [`Send`] variant of the [`ClientRequest`]
    Send = 0x00,

    /// Value tag representing [`Reply`] variant of the [`ClientRequest`]
    Reply = 0x01,

    /// Value tag representing [`ReplyWithSurb`] variant of the [`ClientRequest`]
    ReplyWithSurb = 0x02,

    /// Value tag representing [`SelfAddress`] variant of the [`ClientRequest`]
    SelfAddress = 0x03,
}

impl TryFrom<u8> for ClientRequestTag {
    type Error = error::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Send as u8) => Ok(Self::Send),
            _ if value == (Self::Reply as u8) => Ok(Self::Reply),
            _ if value == (Self::ReplyWithSurb as u8) => Ok(Self::ReplyWithSurb),
            _ if value == (Self::SelfAddress as u8) => Ok(Self::SelfAddress),
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
    Send {
        recipient: Recipient,
        message: Vec<u8>,
        reply_surbs: u32,
    },
    // TODO: another variant to send anonymously WITHOUT attaching surbs (you may think receiver has enough)
    // (you need to attach the tag)
    Reply {
        message: Vec<u8>,
        sender_tag: AnonymousSenderTag,
    },
    ReplyWithSurb {
        message: Vec<u8>,
        reply_surb: ReplySurb,
    },
    SelfAddress,
}

// we could have been parsing it directly TryFrom<WsMessage>, but we want to retain
// information about whether it came from binary or text to send appropriate response back
impl ClientRequest {
    // SEND_REQUEST_TAG || reply_surbs || recipient || data_len || data
    fn serialize_send(recipient: Recipient, data: Vec<u8>, reply_surbs: u32) -> Vec<u8> {
        let data_len_bytes = (data.len() as u64).to_be_bytes();
        std::iter::once(ClientRequestTag::Send as u8)
            .chain(reply_surbs.to_be_bytes().into_iter())
            .chain(recipient.to_bytes().into_iter()) // will not be length prefixed because the length is constant
            .chain(data_len_bytes.into_iter())
            .chain(data.into_iter())
            .collect()
    }

    // SEND_REQUEST_TAG || with_reply || recipient || data_len || data
    fn deserialize_send(b: &[u8]) -> Result<Self, error::Error> {
        // we need to have at least 1 (tag) + 4 (num surbs) + Recipient::LEN + sizeof<u64> bytes
        if b.len() < 2 + Recipient::LEN + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'send'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::Send as u8);

        let reply_surbs = u32::from_be_bytes([b[1], b[2], b[3], b[4]]);

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[5..5 + Recipient::LEN]);
        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(error::Error::new(
                    ErrorKind::MalformedRequest,
                    format!("malformed recipient: {:?}", err),
                ))
            }
        };

        let data_len_bytes = &b[5 + Recipient::LEN..5 + Recipient::LEN + size_of::<u64>()];
        let data_len = u64::from_be_bytes(data_len_bytes.try_into().unwrap());
        let data = &b[5 + Recipient::LEN + size_of::<u64>()..];
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
            reply_surbs,
            recipient,
            message: data.to_vec(),
        })
    }

    // REPLY_REQUEST_TAG || SENDER_TAG || message_len || message
    fn serialize_reply(message: Vec<u8>, sender_tag: AnonymousSenderTag) -> Vec<u8> {
        let message_len_bytes = (message.len() as u64).to_be_bytes();
        std::iter::once(ClientRequestTag::Reply as u8)
            .chain(sender_tag.into_iter())
            .chain(message_len_bytes.into_iter())
            .chain(message.into_iter())
            .collect()
    }

    // REPLY_REQUEST_TAG || SENDER_TAG || message_len || message]
    fn deserialize_reply(b: &[u8]) -> Result<Self, error::Error> {
        if b.len() < 1 + SENDER_TAG_SIZE + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'reply'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::Reply as u8);

        // the unwrap here is fine as we're definitely using exactly 32 bytes
        let sender_tag = b[1..1 + SENDER_TAG_SIZE].try_into().unwrap();

        let message_len = u64::from_be_bytes(
            b[1 + SENDER_TAG_SIZE..1 + SENDER_TAG_SIZE + size_of::<u64>()]
                .try_into()
                .unwrap(),
        );
        let message = &b[1 + SENDER_TAG_SIZE + size_of::<u64>()..];
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
        })
    }

    // REPLY_REQUEST_TAG || surb_len || surb || message_len || message
    fn serialize_reply_with_surb(message: Vec<u8>, reply_surb: ReplySurb) -> Vec<u8> {
        let reply_surb_bytes = reply_surb.to_bytes();
        let surb_len_bytes = (reply_surb_bytes.len() as u64).to_be_bytes();
        let message_len_bytes = (message.len() as u64).to_be_bytes();

        std::iter::once(ClientRequestTag::ReplyWithSurb as u8)
            .chain(surb_len_bytes.into_iter())
            .chain(reply_surb_bytes.into_iter())
            .chain(message_len_bytes.into_iter())
            .chain(message.into_iter())
            .collect()
    }

    // REPLY_REQUEST_TAG || surb_len || surb || message_len || message
    fn deserialize_reply_with_surb(b: &[u8]) -> Result<Self, error::Error> {
        // we need to have at the very least 2 * sizeof<u64> bytes (in case, for some peculiar reason
        // message and reply surb were 0 len - the request would still be malformed, but would in theory
        // be parse-able)
        if b.len() < 1 + 2 * size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortRequest,
                "not enough data provided to recover 'reply with surb'".to_string(),
            ));
        }

        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ClientRequestTag::ReplyWithSurb as u8);

        let reply_surb_len =
            u64::from_be_bytes(b[1..1 + size_of::<u64>()].as_ref().try_into().unwrap());

        // make sure we won't go out of bounds here
        if reply_surb_len > (b.len() - 1 + 2 * size_of::<u64>()) as u64 {
            return Err(error::Error::new(
                ErrorKind::MalformedRequest,
                format!(
                    "not enough data to recover reply surb with specified length {}",
                    reply_surb_len
                ),
            ));
        }

        let surb_bound = 1 + size_of::<u64>() + reply_surb_len as usize;

        let reply_surb_bytes = &b[1 + size_of::<u64>()..surb_bound];
        let reply_surb = match ReplySurb::from_bytes(reply_surb_bytes) {
            Ok(reply_surb) => reply_surb,
            Err(err) => {
                return Err(error::Error::new(
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
            return Err(error::Error::new(
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

        Ok(ClientRequest::ReplyWithSurb {
            reply_surb,
            message: message.to_vec(),
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

    pub fn serialize(self) -> Vec<u8> {
        match self {
            ClientRequest::Send {
                recipient,
                message,
                reply_surbs,
            } => Self::serialize_send(recipient, message, reply_surbs),

            ClientRequest::Reply {
                message,
                sender_tag,
            } => Self::serialize_reply(message, sender_tag),

            ClientRequest::ReplyWithSurb {
                message,
                reply_surb,
            } => Self::serialize_reply_with_surb(message, reply_surb),

            ClientRequest::SelfAddress => Self::serialize_self_address(),
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
            ClientRequestTag::Reply => Self::deserialize_reply(b),
            ClientRequestTag::ReplyWithSurb => Self::deserialize_reply_with_surb(b),
            ClientRequestTag::SelfAddress => Self::deserialize_self_address(b),
        }
    }

    pub fn try_from_binary(raw_req: Vec<u8>) -> Result<Self, error::Error> {
        Self::deserialize(&raw_req)
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

        let send_request_no_surb = ClientRequest::Send {
            recipient,
            message: b"foomp".to_vec(),
            reply_surbs: 0,
        };

        let bytes = send_request_no_surb.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Send {
                recipient,
                message,
                reply_surbs,
            } => {
                assert_eq!(recipient.to_string(), recipient_string);
                assert_eq!(message, b"foomp".to_vec());
                assert_eq!(reply_surbs, 0)
            }
            _ => unreachable!(),
        }

        let send_request_surb = ClientRequest::Send {
            recipient,
            message: b"foomp".to_vec(),
            reply_surbs: 42,
        };

        let bytes = send_request_surb.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::Send {
                recipient,
                message,
                reply_surbs,
            } => {
                assert_eq!(recipient.to_string(), recipient_string);
                assert_eq!(message, b"foomp".to_vec());
                assert_eq!(reply_surbs, 42)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn reply_request_serialization_works() {
        unimplemented!()
    }

    #[test]
    fn reply_with_surb_request_serialization_works() {
        let reply_surb_string = "CjfVbHbfAjbC3W1BvNHGXmM8KNAnDNYGaHMLqVDxRYeo352csAihstup9bvqXam4dTWgfHak6KYwL9STaxWJ47E8XFZbSEvs7hEsfCkxr6K9WJuSBPK84GDDEvad8ZAuMCoaXsAd5S2Lj9a5eYyzG4SL1jHzhSMni55LyJwumxo1ZTGZNXggxw1RREosvyzNrW9Rsi3owyPqLCwXpiei2tHZty8w8midVvg8vDa7ZEJD842CLv8D4ohynSG7gDpqTrhkRaqYAuz7dzqNbMXLJRM7v823Jn16fA1L7YQxmcaUdUigyRSgTdb4i9ebiLGSyJ1iDe6Acz613PQZh6Ua3bZ2zVKq3dSycpDm9ngarRK4zJrAaUxRkdih8YzW3BY4nL9eqkfKA4N1TWCLaRU7zpSaf8yMEwrAZReU3d5zLV8c5KBfa2w8R5anhQeBojduZEGEad8kkHuKU52Zg93FeWHvH1qgZaEJMHH4nN7gKXz9mvWDhYwyF4vt3Uy2NhCHC3N5pL1gMme27YcoPcTEia1fxKZtnt6rtEozzTrAgCJGswigkFbkafiV5QaJwLKTUxtzhkZ57eEuLPte9UvJHzhhXUQ2CV7R2BUkJjYZy3Zsx6YYvdYWiAFFkWUwNEGA4QpShUHciBfsQVHQ7pN41YcyYUhbywQDFnTVgEmdUZ1XCBi3gyK5U3tDQmFzP1u9m3mWrUA8qB9mRDE7ptNDm5c3c1458L6uXLUth7sdMaa1Was5LCmCdmNDtvNpCDAEt1in6q6mrZFR85aCSU9b1baNGwZoCqPpPvydkVe63gXWoi8ebvdyxARrqACFrSB3ZdY3uJBw8CTMNkKK6MvcefMkSVVsbLd36TQAtYSCqrpiMc5dQuKcEu5QfciwvWYXYx8WFNAgKwP2mv49KCTvfozNDUCbjzDwSx92Zv5zjG8HbFpB13bY9UZGeyTPvv7gGxCzjGjJGbW6FRAheRQaaje5fUgCNM95Tv7wBmAMRHHFgWafeK1sdFH7dtCX9u898HucGTaboSKLsVh8J78gbbkHErwjMh7y9YRkceq5TTYS5da4kHnyNKYWSbxgZrmFg44XGKoeYcqoHB3XTZrdsf7F5fFeNwnihkmADvhAcaxXUmVqq4rQFZH84a1iC3WBWXYcqiZH2L7ujGWV7mMDT4HBEerDYjc8rNY4xGTPfivCrBCJW1i14aqW8xRdsdgTM88eTksvC3WPJLJ7iMzfKXeL7fMW1Ek6QGyQtLBW98vEESpdcDg6DeZ5rMz6VqjTGGqcCaFGfHoqtfxMDaBAEsyQ8h7XDX6dg1wq9wH6j4Tw7Tj1MEv1b8uj5NJkozZdzVdYA2QyE2Dp8vuurQG6uVdTDNww2d88RBQ8sVgjxN8gR45y4woJLhFAaNTAtrY6wDTxyXST13ni6oyqdYxjFVk9Am4v3DzH7Y2K8iRVSHfTk4FRbPULyaeK6wt2anvMJH1XdvVRgc14h67MnBxMgMD1UFk8AErN7CDj26fppe3c5G6KozJe4cSqQUGbBjVzBnrHCruqrfZBn5hNZHTV37bQiomqhRQXohxhuKEnNrGbAe1xNvJr9X";
        let reply_surb = ReplySurb::from_base58_string(reply_surb_string).unwrap();
        let reply_request = ClientRequest::ReplyWithSurb {
            message: b"foomp".to_vec(),
            reply_surb,
        };

        let bytes = reply_request.serialize();
        let recovered = ClientRequest::deserialize(&bytes).unwrap();
        match recovered {
            ClientRequest::ReplyWithSurb {
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
}
