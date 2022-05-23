// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::networking::error::NetworkingError;
use crate::networking::PROTOCOL_VERSION;
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io;
use std::time::Duration;
use thiserror::Error;

// I left a sample `NewDealingMessage` to show how it was originally implemented

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OffchainMessage {
    // you'd add something like this:
    // NewDealing {
    //     id: u64,
    //     message: NewDealingMessage,
    // },
    ErrorResponse {
        id: Option<u64>,
        message: ErrorResponseMessage,
    },
}

impl Display for OffchainMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffchainMessage ")?;
        match self {
            // OffchainDkgMessage::NewDealing { id, message } => {
            //     write!(f, "with id {} and message: {}", id, message)
            // }
            OffchainMessage::ErrorResponse { id, message } => {
                write!(f, "with id {:?} and message: {}", id, message)
            }
        }
    }
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct NewDealingMessage {
//     pub epoch_id: u32,
//     pub dealing_bytes: Vec<u8>,
//     pub dealer_signature: identity::Signature,
// }
// impl Display for NewDealingMessage {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "NewDealingMessage for epoch {} with length {}",
//             self.epoch_id,
//             self.dealing_bytes.len()
//         )
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum ErrorResponseMessage {
    #[error("{typ} is not a valid request type")]
    InvalidRequest { typ: String },

    #[error("This request failed to get resolved within {} seconds", .timeout.as_secs())]
    Timeout { timeout: Duration },
}

impl OffchainMessage {
    pub(crate) fn new_error_response(id: Option<u64>, message: ErrorResponseMessage) -> Self {
        OffchainMessage::ErrorResponse { id, message }
    }

    pub(crate) fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, NetworkingError> {
        Ok(bincode::deserialize(&bytes)?)
    }

    pub(crate) fn try_to_bytes(&self) -> Result<Vec<u8>, NetworkingError> {
        Ok(bincode::serialize(&self)?)
    }

    fn frame(&self) -> Result<FramedOffchainDkgMessage, NetworkingError> {
        let payload = self.try_to_bytes()?;
        Ok(FramedOffchainDkgMessage {
            header: Header {
                payload_length: payload.len() as u64,
                protocol_version: PROTOCOL_VERSION,
            },
            payload,
        })
    }

    pub(crate) fn encode(&self, dst: &mut BytesMut) -> Result<(), NetworkingError> {
        dst.put(self.frame()?.into_bytes().as_ref());
        Ok(())
    }
}

struct FramedOffchainDkgMessage {
    header: Header,
    payload: Vec<u8>,
}

impl FramedOffchainDkgMessage {
    fn into_bytes(mut self) -> Vec<u8> {
        let mut header_bytes = self.header.into_bytes();
        let mut out = Vec::with_capacity(header_bytes.len() + self.payload.len());

        out.append(&mut header_bytes);
        out.append(&mut self.payload);
        out
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Header {
    pub(crate) payload_length: u64,
    pub(crate) protocol_version: u32,
}

impl Header {
    pub(crate) const LEN: usize = 12;

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::LEN);
        out.extend_from_slice(&self.payload_length.to_be_bytes());
        out.extend_from_slice(&self.protocol_version.to_be_bytes());

        debug_assert_eq!(Self::LEN, out.len());
        out
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, NetworkingError> {
        if bytes.len() != Self::LEN {
            return Err(NetworkingError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "OffchainMessageType::Header: got {} bytes, expected: {}",
                    bytes.len(),
                    Self::LEN
                ),
            )));
        }
        Ok(Header {
            payload_length: u64::from_be_bytes(bytes[..8].try_into().unwrap()),
            protocol_version: u32::from_be_bytes(bytes[8..].try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::PROTOCOL_VERSION;

    #[test]
    fn header_deserialization() {
        let valid_header = Header {
            payload_length: 1234,
            protocol_version: PROTOCOL_VERSION,
        };

        let bytes = valid_header.into_bytes();
        assert_eq!(valid_header, Header::try_from_bytes(&bytes).unwrap())
    }
}
