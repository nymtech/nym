// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::helpers::BinaryData;
use crate::{GatewayRequestsError, SharedGatewayKey};
use strum::FromRepr;
use tungstenite::Message;

#[non_exhaustive]
pub enum BinaryResponse {
    PushedMixMessage { message: Vec<u8> },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, FromRepr, PartialEq)]
#[non_exhaustive]
pub enum BinaryResponseKind {
    PushedMixMessage = 1,
}

impl BinaryResponse {
    pub fn kind(&self) -> BinaryResponseKind {
        match self {
            BinaryResponse::PushedMixMessage { .. } => BinaryResponseKind::PushedMixMessage,
        }
    }

    pub fn from_plaintext(
        kind: BinaryResponseKind,
        plaintext: &[u8],
    ) -> Result<Self, GatewayRequestsError> {
        match kind {
            BinaryResponseKind::PushedMixMessage => Ok(BinaryResponse::PushedMixMessage {
                message: plaintext.to_vec(),
            }),
        }
    }

    pub fn try_from_encrypted_tagged_bytes(
        bytes: Vec<u8>,
        shared_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        BinaryData::from_raw(&bytes, shared_key)?.into_response(shared_key)
    }

    pub fn into_encrypted_tagged_bytes(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        let kind = self.kind();

        let plaintext = match self {
            BinaryResponse::PushedMixMessage { message } => message,
        };

        BinaryData::make_encrypted_blob(kind as u8, &plaintext, shared_key)
    }

    pub fn into_ws_message(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Message, GatewayRequestsError> {
        // all variants are currently encrypted
        let blob = match self {
            BinaryResponse::PushedMixMessage { .. } => {
                self.into_encrypted_tagged_bytes(shared_key)?
            }
        };

        Ok(Message::Binary(blob))
    }
}
