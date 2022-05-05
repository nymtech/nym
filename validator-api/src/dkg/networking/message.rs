// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::dkg::networking::PROTOCOL_VERSION;
use crate::dkg::state::ReceivedDealing;
use bytes::{BufMut, BytesMut};
use crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub enum OffchainDkgMessage {
    NewDealing {
        id: u64,
        message: NewDealingMessage,
    },
    RemoteDealingRequest {
        id: u64,
        message: RemoteDealingRequestMessage,
    },
    RemoteDealingResponse {
        id: u64,
        message: RemoteDealingResponseMessage,
    },
    ErrorResponse {
        id: Option<u64>,
        message: ErrorResponseMessage,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewDealingMessage {
    pub epoch_id: u32,
    // we keep the dealing in its serialized state as that's what is being signed (and hashed)
    // so that it's easier to verify
    pub dealing_bytes: Vec<u8>,
    pub dealer_signature: identity::Signature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteDealingRequestMessage {
    pub epoch_id: u32,
    pub dealer: identity::PublicKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RemoteDealingResponseMessage {
    Available { dealing: ReceivedDealing },
    Unavailable,
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum ErrorResponseMessage {
    #[error("Received request for epoch: {requested}, while the current epoch is {current}")]
    InvalidEpoch { current: u32, requested: u32 },

    #[error("This sender is not a known dealer for this DKG epoch. Epoch: {epoch_id}, sender: {sender_address}")]
    UnknownDealer {
        sender_address: SocketAddr,
        epoch_id: u32,
    },

    #[error("{typ} is not a valid request type")]
    InvalidRequest { typ: String },

    #[error("This request failed to get resolved within {} seconds", .timeout.as_secs())]
    Timeout { timeout: Duration },
}

impl OffchainDkgMessage {
    pub(crate) fn new_error_response(
        id: Option<u64>,
        message: ErrorResponseMessage,
    ) -> OffchainDkgMessage {
        OffchainDkgMessage::ErrorResponse { id, message }
    }

    pub(crate) fn new_remote_dealing_response(
        id: u64,
        dealing: Option<ReceivedDealing>,
    ) -> OffchainDkgMessage {
        let message = match dealing {
            Some(dealing) => RemoteDealingResponseMessage::Available { dealing },
            None => RemoteDealingResponseMessage::Unavailable,
        };

        OffchainDkgMessage::RemoteDealingResponse { id, message }
    }

    pub(crate) fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, DkgError> {
        Ok(bincode::deserialize(&bytes)?)
    }

    pub(crate) fn try_to_bytes(&self) -> Result<Vec<u8>, DkgError> {
        Ok(bincode::serialize(&self)?)
    }

    fn frame(self) -> Result<FramedOffchainDkgMessage, DkgError> {
        let payload = self.try_to_bytes()?;
        Ok(FramedOffchainDkgMessage {
            header: Header {
                payload_length: payload.len() as u64,
                protocol_version: PROTOCOL_VERSION,
            },
            payload,
        })
    }

    pub(crate) fn encode(self, dst: &mut BytesMut) -> Result<(), DkgError> {
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

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        if bytes.len() != Self::LEN {
            return Err(DkgError::Networking(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "OffchainDkgMessageType::Header: got {} bytes, expected: {}",
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
    use crate::dkg::networking::PROTOCOL_VERSION;

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
