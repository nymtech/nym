// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::psq::{PSQ_MSG2_SIZE, psq_msg1_size};
use nym_kkt::context::KKTMode;
use nym_kkt_ciphersuite::KEM;
use nym_lp_transport::LpTransportError;
use nym_lp_transport::traits::HandshakeMessage;
use std::ops::Deref;

pub struct KKTRequest(nym_kkt::message::KKTRequest);

impl From<nym_kkt::message::KKTRequest> for KKTRequest {
    fn from(request: nym_kkt::message::KKTRequest) -> Self {
        KKTRequest(request)
    }
}

impl From<KKTRequest> for nym_kkt::message::KKTRequest {
    fn from(request: KKTRequest) -> Self {
        request.0
    }
}

impl HandshakeMessage for KKTRequest {
    fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }

    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, LpTransportError> {
        Ok(KKTRequest(
            nym_kkt::message::KKTRequest::try_from_bytes(&bytes)
                .map_err(|err| LpTransportError::MalformedPacket(err.to_string()))?,
        ))
    }

    fn expected_size(mode: KKTMode, expected_kem: KEM, payload_size: usize) -> usize {
        nym_kkt::message::KKTRequest::size_excluding_payload(mode, expected_kem) + payload_size
    }

    fn response_size(&self, expected_kem: KEM, payload_size: usize) -> Option<usize> {
        Some(nym_kkt::message::KKTResponse::size_excluding_payload(expected_kem) + payload_size)
    }
}

pub struct KKTResponse(nym_kkt::message::KKTResponse);

impl From<nym_kkt::message::KKTResponse> for KKTResponse {
    fn from(request: nym_kkt::message::KKTResponse) -> Self {
        KKTResponse(request)
    }
}

impl From<KKTResponse> for nym_kkt::message::KKTResponse {
    fn from(request: KKTResponse) -> Self {
        request.0
    }
}

impl HandshakeMessage for KKTResponse {
    fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }

    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, LpTransportError> {
        Ok(KKTResponse(nym_kkt::message::KKTResponse::from_bytes(
            bytes,
        )))
    }

    fn expected_size(_: KKTMode, expected_kem: KEM, payload_size: usize) -> usize {
        nym_kkt::message::KKTResponse::size_excluding_payload(expected_kem) + payload_size
    }

    fn response_size(&self, expected_kem: KEM, payload_size: usize) -> Option<usize> {
        Some(psq_msg1_size(expected_kem) + payload_size)
    }
}

pub struct PSQMsg1(Vec<u8>);

impl Deref for PSQMsg1 {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PSQMsg1 {
    pub fn new(bytes: Vec<u8>) -> Self {
        PSQMsg1(bytes)
    }
}

impl HandshakeMessage for PSQMsg1 {
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, LpTransportError> {
        Ok(PSQMsg1(bytes))
    }

    fn expected_size(_: KKTMode, expected_kem: KEM, payload_size: usize) -> usize {
        psq_msg1_size(expected_kem) + payload_size
    }

    fn response_size(&self, _: KEM, payload_size: usize) -> Option<usize> {
        Some(PSQ_MSG2_SIZE + payload_size)
    }
}

pub struct PSQMsg2(Vec<u8>);

impl Deref for PSQMsg2 {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PSQMsg2 {
    pub fn new(bytes: Vec<u8>) -> Self {
        PSQMsg2(bytes)
    }
}

impl HandshakeMessage for PSQMsg2 {
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, LpTransportError> {
        Ok(PSQMsg2(bytes))
    }

    fn expected_size(_: KKTMode, _: KEM, payload_size: usize) -> usize {
        PSQ_MSG2_SIZE + payload_size
    }

    fn response_size(&self, _: KEM, _: usize) -> Option<usize> {
        None
    }
}
