// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::helpers::BinaryData;
use crate::{GatewayRequestsError, SharedGatewayKey};
use nym_sphinx::forwarding::packet::MixPacket;
use strum::FromRepr;
use tungstenite::Message;

// in legacy mode requests use zero IV without
pub enum BinaryRequest {
    ForwardSphinx { packet: MixPacket },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, FromRepr, PartialEq)]
#[non_exhaustive]
pub enum BinaryRequestKind {
    ForwardSphinx = 1,
}

// Right now the only valid `BinaryRequest` is a request to forward a sphinx packet.
// It is encrypted using the derived shared key between client and the gateway. Thanks to
// randomness inside the sphinx packet themselves (even via the same route), the 0s IV can be used here.
// HOWEVER, NOTE: If we introduced another 'BinaryRequest', we must carefully examine if a 0s IV
// would work there.
impl BinaryRequest {
    pub fn kind(&self) -> BinaryRequestKind {
        match self {
            BinaryRequest::ForwardSphinx { .. } => BinaryRequestKind::ForwardSphinx,
        }
    }

    pub fn from_plaintext(
        kind: BinaryRequestKind,
        plaintext: &[u8],
    ) -> Result<Self, GatewayRequestsError> {
        match kind {
            BinaryRequestKind::ForwardSphinx => {
                let packet = MixPacket::try_from_bytes(plaintext)?;
                Ok(BinaryRequest::ForwardSphinx { packet })
            }
        }
    }

    pub fn try_from_encrypted_tagged_bytes(
        bytes: Vec<u8>,
        shared_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        BinaryData::from_raw(&bytes, shared_key)?.into_request(shared_key)
    }

    pub fn into_encrypted_tagged_bytes(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        let kind = self.kind();

        let plaintext = match self {
            BinaryRequest::ForwardSphinx { packet } => packet.into_bytes()?,
        };

        BinaryData::make_encrypted_blob(kind as u8, &plaintext, shared_key)
    }

    pub fn into_ws_message(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Message, GatewayRequestsError> {
        // all variants are currently encrypted
        let blob = match self {
            BinaryRequest::ForwardSphinx { .. } => self.into_encrypted_tagged_bytes(shared_key)?,
        };

        Ok(Message::Binary(blob))
    }
}
