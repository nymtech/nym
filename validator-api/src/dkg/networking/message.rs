// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use bytes::{BufMut, BytesMut};
use crypto::asymmetric::identity;
use dkg::Dealing;

pub struct NewDealingMessage {
    epoch_id: u32,
    // we keep the dealing in its serialized state as that's what is being signed (and hashed)
    // so that it's easier to verify
    dealing_bytes: Vec<u8>,
    dealer_signature: identity::Signature,
}

pub struct RemoteDealingRequestMessage {
    epoch_id: u32,
    dealer: identity::PublicKey,
}

pub enum RemoteDealingResponseMessage {
    Available {
        epoch_id: u32,
        dealing: Box<Dealing>,
        dealer_signature: identity::Signature,
    },
    Unavailable,
}

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
}

impl OffchainDkgMessage {
    fn frame(self) -> FramedOffchainDkgMessage {
        todo!()
    }

    pub(crate) fn encode(self, dst: &mut BytesMut) {
        dst.put(self.frame().into_bytes().as_ref());
    }

    pub(crate) fn try_from_bytes(
        bytes: Vec<u8>,
        expected_type: OffchainDkgMessageType,
    ) -> Result<Self, DkgError> {
        todo!()
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OffchainDkgMessageType {
    NewDealing = 0,
    RemoteDealingRequest = 1,

    RemoteDealingResponse = 128,
    ErrorResponse = 255,
}

pub struct InvalidDkgMessageType(u8);

impl TryFrom<u8> for OffchainDkgMessageType {
    type Error = InvalidDkgMessageType;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == (OffchainDkgMessageType::NewDealing as u8) => Ok(Self::NewDealing),
            _ if value == (OffchainDkgMessageType::RemoteDealingRequest as u8) => {
                Ok(Self::RemoteDealingRequest)
            }
            _ if value == (OffchainDkgMessageType::RemoteDealingResponse as u8) => {
                Ok(Self::RemoteDealingResponse)
            }
            _ if value == (OffchainDkgMessageType::ErrorResponse as u8) => Ok(Self::ErrorResponse),
            t => Err(InvalidDkgMessageType(t)),
        }
    }
}

struct FramedOffchainDkgMessage {
    header: Header,
    payload: Vec<u8>,
}

impl FramedOffchainDkgMessage {
    fn into_bytes(mut self) -> Vec<u8> {
        let mut header_bytes = self.header.to_bytes();
        let mut out = Vec::with_capacity(header_bytes.len() + self.payload.len());

        out.append(&mut header_bytes);
        out.append(&mut self.payload);
        out
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}

pub(crate) struct Header {
    pub(crate) message_type: OffchainDkgMessageType,
    pub(crate) payload_length: u64,
    pub(crate) protocol_version: u32,
}

impl Header {
    pub(crate) const LEN: usize = 13;

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::LEN);
        out.push(self.message_type as u8);
        out.extend_from_slice(&self.payload_length.to_be_bytes());
        out.extend_from_slice(&self.protocol_version.to_be_bytes());

        debug_assert_eq!(Self::LEN, out.len());
        out
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        todo!()
    }
}
