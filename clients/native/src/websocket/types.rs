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

use nymsphinx::addressing::clients::Recipient;
use nymsphinx::receiver::ReconstructedMessage;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientRequest {
    Send {
        message: String,
        recipient: String,
        // Perhaps we could change it to a number to indicate how many reply_surbs we want to include?
        with_reply_surb: bool,
    },
    GetClients,
    SelfAddress,
    // Reply {
    //     message: String,
    //     #[allow(non_snake_case)]
    //     reply_SURB: String,
    // }
}

impl TryFrom<String> for ClientRequest {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl Into<Message> for ClientRequest {
    fn into(self) -> Message {
        let str_req = serde_json::to_string(&self).unwrap();
        Message::Text(str_req)
    }
}

pub enum BinaryClientRequest {
    Send {
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
    },
    // Reply {
    //     message: Vec<u8>,
    //     #[allow(non_snake_case)]
    //     reply_SURB: ReplySURB,
    // },
}

impl BinaryClientRequest {
    // TODO: perhaps do it the proper way and introduce an error type
    pub fn try_from_bytes(req: &[u8]) -> Option<Self> {
        if req.len() < Recipient::LEN + 1 {
            return None;
        }

        let with_reply_surb = match req[0] {
            n if n == 1 => true,
            n if n == 0 => false,
            _ => return None, // we only 'accept' 0 or 1 byte here
        };

        let mut recipient_bytes = [0u8; Recipient::LEN];
        recipient_bytes.copy_from_slice(&req[1..Recipient::LEN + 1]);
        let recipient = Recipient::try_from_bytes(recipient_bytes).ok()?;

        Some(BinaryClientRequest::Send {
            recipient,
            data: req[1 + Recipient::LEN..].to_vec(),
            with_reply_surb,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Send {
                recipient,
                data,
                with_reply_surb,
            } => std::iter::once(if with_reply_surb { 1u8 } else { 0u8 })
                .chain(recipient.into_bytes().iter().cloned())
                .chain(data.into_iter())
                .collect(),
        }
    }
}

impl Into<Message> for BinaryClientRequest {
    fn into(self) -> Message {
        Message::Binary(self.into_bytes())
    }
}

// TODO: it's very likely this will be renamed and will also be used to send replies via SURBs
// but for time being let's just leave it like that

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedMessage {
    message: String,
    reply_surb: Option<String>,
}

impl<'a> TryFrom<&'a ReconstructedMessage> for ReceivedMessage {
    type Error = std::str::Utf8Error;

    fn try_from(reconstructed_message: &ReconstructedMessage) -> Result<Self, Self::Error> {
        Ok(ReceivedMessage {
            message: std::str::from_utf8(&reconstructed_message.message)?.to_string(),
            reply_surb: reconstructed_message
                .reply_SURB
                .as_ref()
                .map(|reply_surb| reply_surb.to_base58_string()),
        })
    }
}

impl ReceivedMessage {
    pub fn to_json(&self) -> String {
        // from the docs:
        // "Serialization can fail if `T`'s implementation of `Serialize` decides to
        // fail, or if `T` contains a map with non-string keys."
        // so under those conditions it's impossible for the serialization to fail.
        serde_json::to_string(&self).expect("json serialization unexpectedly failed!")
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerResponse {
    Send,
    Received { messages: Vec<ReceivedMessage> },
    GetClients { clients: Vec<String> },
    SelfAddress { address: String },
    Error { message: String },
}

impl ServerResponse {
    pub fn new_error<S: Into<String>>(msg: S) -> Self {
        ServerResponse::Error {
            message: msg.into(),
        }
    }
}

impl TryFrom<String> for ServerResponse {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, <ServerResponse as TryFrom<String>>::Error> {
        serde_json::from_str(&msg)
    }
}

impl Into<Message> for ServerResponse {
    fn into(self) -> Message {
        // it should be safe to call `unwrap` here as the message is generated by the server
        // so if it fails (and consequently panics) it's a bug that should be resolved
        let str_res = serde_json::to_string(&self).unwrap();
        Message::Text(str_res)
    }
}
