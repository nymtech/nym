// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RegistrationHandshake {
    HandshakePayload {
        #[serde(default)]
        protocol_version: Option<u8>,
        data: Vec<u8>,
    },
    HandshakeError {
        message: String,
    },
}

impl RegistrationHandshake {
    pub fn new_payload(data: Vec<u8>, protocol_version: u8) -> Self {
        RegistrationHandshake::HandshakePayload {
            protocol_version: Some(protocol_version),
            data,
        }
    }

    pub fn new_error<S: Into<String>>(message: S) -> Self {
        RegistrationHandshake::HandshakeError {
            message: message.into(),
        }
    }
}

impl FromStr for RegistrationHandshake {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl TryFrom<String> for RegistrationHandshake {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, serde_json::Error> {
        msg.parse()
    }
}

impl TryInto<String> for RegistrationHandshake {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientControlRequest;

    #[test]
    fn handshake_payload_can_be_deserialized_into_register_handshake_init_request() {
        let handshake_data = vec![1, 2, 3, 4, 5, 6];
        let handshake_payload_with_protocol = RegistrationHandshake::HandshakePayload {
            protocol_version: Some(42),
            data: handshake_data.clone(),
        };
        let serialized = serde_json::to_string(&handshake_payload_with_protocol).unwrap();
        let deserialized = ClientControlRequest::try_from(serialized).unwrap();

        match deserialized {
            ClientControlRequest::RegisterHandshakeInitRequest {
                protocol_version,
                data,
            } => {
                assert_eq!(protocol_version, Some(42));
                assert_eq!(data, handshake_data)
            }
            _ => unreachable!("this branch shouldn't have been reached!"),
        }

        let handshake_payload_without_protocol = RegistrationHandshake::HandshakePayload {
            protocol_version: None,
            data: handshake_data.clone(),
        };
        let serialized = serde_json::to_string(&handshake_payload_without_protocol).unwrap();
        let deserialized = ClientControlRequest::try_from(serialized).unwrap();

        match deserialized {
            ClientControlRequest::RegisterHandshakeInitRequest {
                protocol_version,
                data,
            } => {
                assert!(protocol_version.is_none());
                assert_eq!(data, handshake_data)
            }
            _ => unreachable!("this branch shouldn't have been reached!"),
        }
    }
}
