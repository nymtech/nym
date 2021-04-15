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

// all variable size data is always prefixed with u64 length
// tags are u8

#![allow(unknown_lints)] // due to using `clippy::branches_sharing_code` which does not exist on `stable` just yet

use crate::error::{self, ErrorKind};
use crate::text::ServerResponseText;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::receiver::ReconstructedMessage;
use std::convert::TryInto;
use std::mem::size_of;

/// Value tag representing [`Error`] variant of the [`ServerResponse`]
pub const ERROR_RESPONSE_TAG: u8 = 0x00;

/// Value tag representing [`Received`] variant of the [`ServerResponse`]
pub const RECEIVED_RESPONSE_TAG: u8 = 0x01;

/// Value tag representing [`SelfAddress`] variant of the [`ServerResponse`]
pub const SELF_ADDRESS_RESPONSE_TAG: u8 = 0x02;

#[derive(Debug)]
pub enum ServerResponse {
    Received(ReconstructedMessage),
    SelfAddress(Recipient),
    Error(error::Error),
}

impl ServerResponse {
    pub fn new_error<S: Into<String>>(message: S) -> Self {
        ServerResponse::Error(error::Error {
            kind: ErrorKind::Other,
            message: message.into(),
        })
    }

    // RECEIVED_RESPONSE_TAG || with_reply || (surb_len || surb) || msg_len || msg
    fn serialize_received(reconstructed_message: ReconstructedMessage) -> Vec<u8> {
        let message_len_bytes = (reconstructed_message.message.len() as u64).to_be_bytes();
        if let Some(reply_surb) = reconstructed_message.reply_surb {
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
    fn deserialize_received(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], RECEIVED_RESPONSE_TAG);

        // we must be able to read at the very least if it has a reply_surb and length of some field
        if b.len() < 2 + size_of::<u64>() {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'received'".to_string(),
            ));
        }

        let with_reply_surb = match b[1] {
            0 => false,
            1 => true,
            n => {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    format!("invalid reply flag {}", n),
                ))
            }
        };

        // this is a false positive as even though the code is the same, it refers to different things
        #[allow(clippy::branches_sharing_code)]
        if with_reply_surb {
            let reply_surb_len =
                u64::from_be_bytes(b[2..2 + size_of::<u64>()].as_ref().try_into().unwrap());

            // make sure we won't go out of bounds here
            if reply_surb_len > (b.len() - 2 + 2 * size_of::<u64>()) as u64 {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    "not enough bytes to read reply_surb bytes!".to_string(),
                ));
            }

            let surb_bound = 2 + size_of::<u64>() + reply_surb_len as usize;

            let reply_surb_bytes = &b[2 + size_of::<u64>()..surb_bound];
            let reply_surb = match ReplySurb::from_bytes(reply_surb_bytes) {
                Ok(reply_surb) => reply_surb,
                Err(err) => {
                    return Err(error::Error::new(
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
                reply_surb: Some(reply_surb),
            }))
        } else {
            let message_len =
                u64::from_be_bytes(b[2..2 + size_of::<u64>()].as_ref().try_into().unwrap());
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

            Ok(ServerResponse::Received(ReconstructedMessage {
                message: message.to_vec(),
                reply_surb: None,
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
    fn deserialize_self_address(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], SELF_ADDRESS_RESPONSE_TAG);

        if b.len() != 1 + Recipient::LEN {
            return Err(error::Error::new(
                ErrorKind::TooShortResponse,
                "not enough data provided to recover 'self_address'".to_string(),
            ));
        }

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&b[1..1 + Recipient::LEN]);

        let recipient = match Recipient::try_from_bytes(recipient_bytes) {
            Ok(recipient) => recipient,
            Err(err) => {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    format!("malformed Recipient: {:?}", err),
                ))
            }
        };

        Ok(ServerResponse::SelfAddress(recipient))
    }

    // ERROR_RESPONSE_TAG || err_code || msg_len || msg
    fn serialize_error(error: error::Error) -> Vec<u8> {
        let message_len_bytes = (error.message.len() as u64).to_be_bytes();
        std::iter::once(ERROR_RESPONSE_TAG)
            .chain(std::iter::once(error.kind as u8))
            .chain(message_len_bytes.iter().cloned())
            .chain(error.message.into_bytes().into_iter())
            .collect()
    }

    // ERROR_RESPONSE_TAG || err_code || msg_len || msg
    fn deserialize_error(b: &[u8]) -> Result<Self, error::Error> {
        // this MUST match because it was called by 'deserialize'
        debug_assert_eq!(b[0], ERROR_RESPONSE_TAG);

        if b.len() < size_of::<u8>() + size_of::<u64>() {
            return Err(error::Error::new(
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

            _ if b[1] == (ErrorKind::Other as u8) => ErrorKind::Other,

            n => {
                return Err(error::Error::new(
                    ErrorKind::MalformedResponse,
                    format!("invalid error code {}", n),
                ))
            }
        };

        let message_len =
            u64::from_be_bytes(b[2..2 + size_of::<u64>()].as_ref().try_into().unwrap());
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
                    format!("malformed error message: {:?}", err),
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
            ServerResponse::SelfAddress(address) => Self::serialize_self_address(address),
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

        let response_tag = b[0];

        // determine what kind of response that is and try to deserialize it
        match response_tag {
            RECEIVED_RESPONSE_TAG => Self::deserialize_received(b),
            SELF_ADDRESS_RESPONSE_TAG => Self::deserialize_self_address(b),
            ERROR_RESPONSE_TAG => Self::deserialize_error(b),
            n => Err(error::Error::new(
                ErrorKind::UnknownResponse,
                format!("type {}", n),
            )),
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
        let reply_surb_string = "CjfVbHbfAjbC3W1BvNHGXmM8KNAnDNYGaHMLqVDxRYeo352csAihstup9bvqXam4dTWgfHak6KYwL9STaxWJ47E8XFZbSEvs7hEsfCkxr6K9WJuSBPK84GDDEvad8ZAuMCoaXsAd5S2Lj9a5eYyzG4SL1jHzhSMni55LyJwumxo1ZTGZNXggxw1RREosvyzNrW9Rsi3owyPqLCwXpiei2tHZty8w8midVvg8vDa7ZEJD842CLv8D4ohynSG7gDpqTrhkRaqYAuz7dzqNbMXLJRM7v823Jn16fA1L7YQxmcaUdUigyRSgTdb4i9ebiLGSyJ1iDe6Acz613PQZh6Ua3bZ2zVKq3dSycpDm9ngarRK4zJrAaUxRkdih8YzW3BY4nL9eqkfKA4N1TWCLaRU7zpSaf8yMEwrAZReU3d5zLV8c5KBfa2w8R5anhQeBojduZEGEad8kkHuKU52Zg93FeWHvH1qgZaEJMHH4nN7gKXz9mvWDhYwyF4vt3Uy2NhCHC3N5pL1gMme27YcoPcTEia1fxKZtnt6rtEozzTrAgCJGswigkFbkafiV5QaJwLKTUxtzhkZ57eEuLPte9UvJHzhhXUQ2CV7R2BUkJjYZy3Zsx6YYvdYWiAFFkWUwNEGA4QpShUHciBfsQVHQ7pN41YcyYUhbywQDFnTVgEmdUZ1XCBi3gyK5U3tDQmFzP1u9m3mWrUA8qB9mRDE7ptNDm5c3c1458L6uXLUth7sdMaa1Was5LCmCdmNDtvNpCDAEt1in6q6mrZFR85aCSU9b1baNGwZoCqPpPvydkVe63gXWoi8ebvdyxARrqACFrSB3ZdY3uJBw8CTMNkKK6MvcefMkSVVsbLd36TQAtYSCqrpiMc5dQuKcEu5QfciwvWYXYx8WFNAgKwP2mv49KCTvfozNDUCbjzDwSx92Zv5zjG8HbFpB13bY9UZGeyTPvv7gGxCzjGjJGbW6FRAheRQaaje5fUgCNM95Tv7wBmAMRHHFgWafeK1sdFH7dtCX9u898HucGTaboSKLsVh8J78gbbkHErwjMh7y9YRkceq5TTYS5da4kHnyNKYWSbxgZrmFg44XGKoeYcqoHB3XTZrdsf7F5fFeNwnihkmADvhAcaxXUmVqq4rQFZH84a1iC3WBWXYcqiZH2L7ujGWV7mMDT4HBEerDYjc8rNY4xGTPfivCrBCJW1i14aqW8xRdsdgTM88eTksvC3WPJLJ7iMzfKXeL7fMW1Ek6QGyQtLBW98vEESpdcDg6DeZ5rMz6VqjTGGqcCaFGfHoqtfxMDaBAEsyQ8h7XDX6dg1wq9wH6j4Tw7Tj1MEv1b8uj5NJkozZdzVdYA2QyE2Dp8vuurQG6uVdTDNww2d88RBQ8sVgjxN8gR45y4woJLhFAaNTAtrY6wDTxyXST13ni6oyqdYxjFVk9Am4v3DzH7Y2K8iRVSHfTk4FRbPULyaeK6wt2anvMJH1XdvVRgc14h67MnBxMgMD1UFk8AErN7CDj26fppe3c5G6KozJe4cSqQUGbBjVzBnrHCruqrfZBn5hNZHTV37bQiomqhRQXohxhuKEnNrGbAe1xNvJr9X";

        let received_with_surb = ServerResponse::Received(ReconstructedMessage {
            message: b"foomp".to_vec(),
            reply_surb: Some(ReplySurb::from_base58_string(reply_surb_string).unwrap()),
        });
        let bytes = received_with_surb.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Received(reconstructed) => {
                assert_eq!(reconstructed.message, b"foomp".to_vec());
                assert_eq!(
                    reconstructed.reply_surb.unwrap().to_base58_string(),
                    reply_surb_string
                )
            }
            _ => unreachable!(),
        }

        let received_without_surb = ServerResponse::Received(ReconstructedMessage {
            message: b"foomp".to_vec(),
            reply_surb: None,
        });
        let bytes = received_without_surb.serialize();
        let recovered = ServerResponse::deserialize(&bytes).unwrap();
        match recovered {
            ServerResponse::Received(reconstructed) => {
                assert_eq!(reconstructed.message, b"foomp".to_vec());
                assert!(reconstructed.reply_surb.is_none())
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn self_address_response_serialization_works() {
        let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
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
