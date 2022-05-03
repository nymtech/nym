// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::dkg::networking::PROTOCOL_VERSION;
use bytes::{BufMut, BytesMut};
use crypto::asymmetric::identity;
use dkg::Dealing;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io;

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
        id: u64,
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
    Available {
        epoch_id: u32,
        #[serde(with = "dealing_bytes")]
        dealing: Box<Dealing>,
        dealer_signature: identity::Signature,
    },
    Unavailable,
}

mod dealing_bytes {
    use dkg::Dealing;
    use serde::de::Error as SerdeError;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_bytes::{ByteBuf as SerdeByteBuf, Bytes as SerdeBytes};

    pub fn serialize<S: Serializer>(val: &Dealing, serializer: S) -> Result<S::Ok, S::Error> {
        SerdeBytes::new(&val.to_bytes()).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Box<Dealing>, D::Error> {
        let bytes = <SerdeByteBuf>::deserialize(deserializer)?;
        let dealing = Dealing::try_from_bytes(bytes.as_ref()).map_err(SerdeError::custom)?;
        Ok(Box::new(dealing))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponseMessage {
    pub reason: ErrorReason,
    pub additional_info: Option<String>,
}

impl ErrorResponseMessage {
    pub fn new(reason: ErrorReason, additional_info: Option<String>) -> Self {
        ErrorResponseMessage {
            reason,
            additional_info,
        }
    }
}

impl Display for ErrorResponseMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorReason {
    InvalidEpoch,
    UnknownDealer,
    InvalidRequest,
    Timeout,
}

impl Display for ErrorReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl OffchainDkgMessage {
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

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        todo!()
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
