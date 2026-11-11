// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::psq::{PSQ_MSG2_SIZE, psq_msg1_size};
use crate::transport::{LpTransportError, traits::HandshakeMessage};
use nym_kkt::context::KKTMode;
use nym_kkt_ciphersuite::KEM;
use nym_lp_data::packet::version;
use std::ops::Deref;

/// Size of the little-endian `u32` frame length a version 2 request carries after its cleartext prefix.
pub(crate) const FRAME_LENGTH_PREFIX_SIZE: usize = 4;

pub struct KKTRequest {
    inner: nym_kkt::message::KKTRequest,
    protocol_version: u8,
}

impl KKTRequest {
    pub fn new(inner: nym_kkt::message::KKTRequest, protocol_version: u8) -> Self {
        KKTRequest {
            inner,
            protocol_version,
        }
    }
}

impl From<KKTRequest> for nym_kkt::message::KKTRequest {
    fn from(request: KKTRequest) -> Self {
        request.inner
    }
}

impl HandshakeMessage for KKTRequest {
    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = self.inner.into_bytes();
        if self.protocol_version < version::V2 {
            return bytes;
        }

        // version 2: the encrypted frame is preceded by its length
        let frame = bytes.split_off(nym_kkt::message::KKTRequest::PLAINTEXT_SIZE);
        bytes.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&frame);
        bytes
    }

    /// Decodes the version 1 layout; the responder reads version 2 in two stages instead.
    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, LpTransportError> {
        Ok(KKTRequest::new(
            nym_kkt::message::KKTRequest::try_from_bytes(&bytes)
                .map_err(|err| LpTransportError::MalformedPacket(err.to_string()))?,
            version::V1,
        ))
    }

    /// Size of the version 1 layout; the responder reads version 2 in two stages instead.
    fn expected_size(mode: KKTMode, expected_kem: KEM, payload_size: usize) -> usize {
        nym_kkt::message::KKTRequest::size_excluding_payload(mode, expected_kem) + payload_size
    }

    fn response_size(&self, expected_kem: KEM) -> Option<usize> {
        Some(nym_kkt::message::KKTResponse::size_excluding_payload(
            expected_kem,
        ))
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

    fn response_size(&self, expected_kem: KEM) -> Option<usize> {
        Some(psq_msg1_size(expected_kem))
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

    fn response_size(&self, _: KEM) -> Option<usize> {
        Some(PSQ_MSG2_SIZE)
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

    fn response_size(&self, _: KEM) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAINTEXT_SIZE: usize = nym_kkt::message::KKTRequest::PLAINTEXT_SIZE;

    fn request_with_frame(frame: Vec<u8>) -> nym_kkt::message::KKTRequest {
        nym_kkt::message::KKTRequest::from_parts(&[7u8; PLAINTEXT_SIZE], frame).unwrap()
    }

    #[test]
    fn a_v1_request_is_the_prefix_and_the_frame() {
        let bytes = KKTRequest::new(request_with_frame(vec![1, 2, 3]), version::V1).into_bytes();

        assert_eq!(&bytes[..PLAINTEXT_SIZE], &[7u8; PLAINTEXT_SIZE]);
        assert_eq!(&bytes[PLAINTEXT_SIZE..], &[1, 2, 3]);
    }

    #[test]
    fn a_v2_request_carries_the_frame_length_after_the_prefix() {
        let bytes = KKTRequest::new(request_with_frame(vec![1, 2, 3]), version::V2).into_bytes();

        assert_eq!(&bytes[..PLAINTEXT_SIZE], &[7u8; PLAINTEXT_SIZE]);
        assert_eq!(
            &bytes[PLAINTEXT_SIZE..PLAINTEXT_SIZE + FRAME_LENGTH_PREFIX_SIZE],
            &3u32.to_le_bytes()
        );
        assert_eq!(
            &bytes[PLAINTEXT_SIZE + FRAME_LENGTH_PREFIX_SIZE..],
            &[1, 2, 3]
        );
    }
}
