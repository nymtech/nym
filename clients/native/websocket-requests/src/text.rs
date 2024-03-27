// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ErrorKind;
use crate::requests::ClientRequest;
use crate::responses::ServerResponse;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use serde::{Deserialize, Serialize};

// local text equivalent of `ClientRequest` for easier serialization + deserialization with serde
// TODO: figure out if there's an easy way to avoid defining it

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum ClientRequestText {
    #[serde(rename_all = "camelCase")]
    Send {
        message: String,
        recipient: String,
        connection_id: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    SendAnonymous {
        recipient: String,
        message: String,
        reply_surbs: u32,
        connection_id: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Reply {
        sender_tag: String,
        message: String,
        connection_id: Option<u64>,
    },
    SelfAddress,
}

impl TryFrom<String> for ClientRequestText {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl TryInto<ClientRequest> for ClientRequestText {
    type Error = crate::error::Error;

    fn try_into(self) -> Result<ClientRequest, Self::Error> {
        match self {
            ClientRequestText::Send {
                message,
                recipient,
                connection_id,
            } => {
                let message_bytes = message.into_bytes();
                let recipient = Recipient::try_from_base58_string(recipient).map_err(|err| {
                    Self::Error::new(ErrorKind::MalformedRequest, err.to_string())
                })?;

                Ok(ClientRequest::Send {
                    message: message_bytes,
                    recipient,
                    connection_id,
                })
            }
            ClientRequestText::SendAnonymous {
                recipient,
                message,
                reply_surbs,
                connection_id,
            } => {
                let message_bytes = message.into_bytes();
                let recipient = Recipient::try_from_base58_string(recipient).map_err(|err| {
                    Self::Error::new(ErrorKind::MalformedRequest, err.to_string())
                })?;
                Ok(ClientRequest::SendAnonymous {
                    recipient,
                    message: message_bytes,
                    reply_surbs,
                    connection_id,
                })
            }
            ClientRequestText::SelfAddress => Ok(ClientRequest::SelfAddress),
            ClientRequestText::Reply {
                sender_tag,
                message,
                connection_id,
            } => {
                let message_bytes = message.into_bytes();
                let sender_tag =
                    AnonymousSenderTag::try_from_base58_string(sender_tag).map_err(|err| {
                        Self::Error::new(ErrorKind::MalformedRequest, err.to_string())
                    })?;

                Ok(ClientRequest::Reply {
                    sender_tag,
                    message: message_bytes,
                    connection_id,
                })
            }
        }
    }
}

// local text equivalent of `ServerResponse` for easier serialization + deserialization with serde
// TODO: figure out if there's an easy way to avoid defining it

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum ServerResponseText {
    #[serde(rename_all = "camelCase")]
    Received {
        message: String,
        sender_tag: Option<String>,
    },
    SelfAddress {
        address: String,
    },
    LaneQueueLength {
        lane: u64,
        queue_length: usize,
    },
    Error {
        message: String,
    },
}

impl TryFrom<String> for ServerResponseText {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, <ServerResponseText as TryFrom<String>>::Error> {
        serde_json::from_str(&msg)
    }
}

impl From<ServerResponseText> for String {
    fn from(res: ServerResponseText) -> Self {
        // per serde_json docs:
        /*
        /// Serialization can fail if `T`'s implementation of `Serialize` decides to
        /// fail, or if `T` contains a map with non-string keys.
         */
        // this is not the case here.
        serde_json::to_string(&res).unwrap()
    }
}

impl From<ServerResponse> for ServerResponseText {
    fn from(resp: ServerResponse) -> Self {
        match resp {
            ServerResponse::Received(reconstructed) => {
                ServerResponseText::Received {
                    // TODO: ask DH what is more appropriate, lossy utf8 conversion or returning error and then
                    // pure binary later
                    message: String::from_utf8_lossy(&reconstructed.message).into_owned(),
                    sender_tag: reconstructed.sender_tag.map(|tag| tag.to_base58_string()),
                }
            }
            ServerResponse::SelfAddress(recipient) => ServerResponseText::SelfAddress {
                address: recipient.to_string(),
            },
            ServerResponse::LaneQueueLength { lane, queue_length } => {
                ServerResponseText::LaneQueueLength { lane, queue_length }
            }
            ServerResponse::Error(err) => ServerResponseText::Error {
                message: err.to_string(),
            },
        }
    }
}
