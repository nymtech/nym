// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::carrier::Carrier;
use crate::context::{KKTContext, KKTMode, KKTRole};
use crate::error::KKTError;
use crate::frame::KKTFrame;
use crate::keys::EncapsulationKey;
use crate::masked_byte::{MASKED_BYTE_LEN, MaskedByte};
use libcrux_chacha20poly1305::TAG_LEN;
use libcrux_psq::handshake::types::{DHKeyPair, DHPrivateKey, DHPublicKey};
use nym_kkt_ciphersuite::{KEM, x25519};

pub struct KKTRequest {
    /// The plaintext part of the request
    pub(crate) plaintext: KKTRequestPlaintext,

    /// Ciphertext of an initial request `KKTFrame`
    pub(crate) encrypted_frame: Vec<u8>,
}

impl KKTRequest {
    // the size of KKTRequest is the plaintext data followed by the frame and the encryption tag
    pub const fn size_excluding_payload(mode: KKTMode, kem: KEM) -> usize {
        KKTRequestPlaintext::SIZE
            + KKTFrame::size_excluding_payload(KKTRole::Initiator, mode, kem)
            + TAG_LEN
    }

    pub fn size(&self) -> usize {
        self.encrypted_frame.len() + KKTRequestPlaintext::SIZE
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        let mut out = self.plaintext.to_bytes();
        out.append(&mut self.encrypted_frame);
        out
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Self, KKTError> {
        if b.len() < x25519::PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN {
            return Err(KKTError::FrameDecodingError {
                info: "the KKTRequest frame has invalid length".to_string(),
            });
        }
        #[allow(clippy::indexing_slicing)]
        let plaintext =
            KKTRequestPlaintext::try_from_bytes(&b[..x25519::PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN])?;

        #[allow(clippy::indexing_slicing)]
        Ok(KKTRequest {
            plaintext,
            encrypted_frame: b[x25519::PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN..].to_vec(),
        })
    }

    /// Size of the cleartext prefix: the initiator's ephemeral key and the masked version.
    pub const PLAINTEXT_SIZE: usize = KKTRequestPlaintext::SIZE;

    /// Unmask the outer protocol version from the cleartext prefix alone, before the frame is read.
    pub fn peek_outer_protocol_version(
        plaintext: &[u8],
        responder_pubkey: &DHPublicKey,
        supported_versions: &[u8],
    ) -> Result<u8, KKTError> {
        let plaintext = KKTRequestPlaintext::try_from_bytes(plaintext)?;
        let mask = plaintext.version_mask(responder_pubkey);
        Ok(plaintext
            .masked_version_bytes
            .unmask_check_version(&mask, supported_versions)?)
    }

    /// Assemble a request from its cleartext prefix and the separately read encrypted frame.
    pub fn from_parts(plaintext: &[u8], encrypted_frame: Vec<u8>) -> Result<Self, KKTError> {
        Ok(KKTRequest {
            plaintext: KKTRequestPlaintext::try_from_bytes(plaintext)?,
            encrypted_frame,
        })
    }
}

pub(crate) struct KKTRequestPlaintext {
    /// Ephemeral Diffie-Hellman public key of the initiator
    pub(crate) dh_pubkey: DHPublicKey,

    /// Masked bytes representing the outer protocol version information
    pub(crate) masked_version_bytes: MaskedByte,
}

impl KKTRequestPlaintext {
    pub const SIZE: usize = x25519::PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN;

    pub(crate) fn new(
        initiator_pubkey: DHPublicKey,
        responder_pubkey: &DHPublicKey,
        outer_protocol_version: u8,
    ) -> Self {
        let mask = Self::create_version_mask(&initiator_pubkey, responder_pubkey);
        let masked_version_bytes = MaskedByte::new(outer_protocol_version, &mask);
        KKTRequestPlaintext {
            dh_pubkey: initiator_pubkey,
            masked_version_bytes,
        }
    }

    pub(crate) fn into_request(
        self,
        carrier: &mut Carrier,
        frame: KKTFrame,
    ) -> Result<KKTRequest, KKTError> {
        let frame_bytes = frame.try_to_bytes()?;
        let frame_ciphertext = carrier.encrypt(&frame_bytes)?;
        Ok(KKTRequest {
            plaintext: self,
            encrypted_frame: frame_ciphertext,
        })
    }

    pub(crate) fn create_version_mask(
        initiator_pubkey: &DHPublicKey,
        responder_pubkey: &DHPublicKey,
    ) -> Vec<u8> {
        let mut mask = Vec::with_capacity(2 * x25519::PUBLIC_KEY_LENGTH);
        mask.extend_from_slice(initiator_pubkey.as_ref());
        mask.extend_from_slice(responder_pubkey.as_ref());
        mask
    }

    fn create_carrier_ctx(
        masked_version: &MaskedByte,
        initiator_pubkey: &DHPublicKey,
        responder_pubkey: &DHPublicKey,
    ) -> Vec<u8> {
        let mut context = Vec::new();
        context.extend_from_slice(masked_version.as_slice());
        context.extend_from_slice(crate::frame::KKT_CARRIER_CONTEXT);
        context.extend_from_slice(initiator_pubkey.as_ref());
        context.extend_from_slice(responder_pubkey.as_ref());
        context
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);
        out.extend_from_slice(self.dh_pubkey.as_ref());
        out.extend_from_slice(self.masked_version_bytes.as_slice());
        out
    }

    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, KKTError> {
        if b.len() != Self::SIZE {
            return Err(KKTError::FrameDecodingError {
                info: "the KKTRequest frame has invalid length".to_string(),
            });
        }
        // SAFETY: we're using exactly 32 byte
        #[allow(clippy::unwrap_used)]
        #[allow(clippy::indexing_slicing)]
        let dh_pubkey =
            DHPublicKey::from_bytes(&b[..x25519::PUBLIC_KEY_LENGTH].try_into().unwrap());
        #[allow(clippy::indexing_slicing)]
        let masked_version_bytes = MaskedByte::try_from(&b[x25519::PUBLIC_KEY_LENGTH..])?;

        Ok(KKTRequestPlaintext {
            dh_pubkey,
            masked_version_bytes,
        })
    }

    pub(crate) fn version_mask(&self, responder_pubkey: &DHPublicKey) -> Vec<u8> {
        Self::create_version_mask(&self.dh_pubkey, responder_pubkey)
    }

    pub(crate) fn derive_initiator_carrier(
        &self,
        initiator_sk: &DHPrivateKey,
        responder_pubkey: &DHPublicKey,
    ) -> Result<Carrier, KKTError> {
        let ctx = Self::create_carrier_ctx(
            &self.masked_version_bytes,
            &self.dh_pubkey,
            responder_pubkey,
        );

        let shared_secret = initiator_sk
            .diffie_hellman(responder_pubkey)
            .map_err(KKTError::shared_secret_derivation_failure)?;

        Ok(Carrier::from_secret_slice(
            shared_secret.as_ref(),
            &ctx,
            true,
        ))
    }

    pub(crate) fn derive_responder_carrier(
        &self,
        responder_keys: &DHKeyPair,
    ) -> Result<Carrier, KKTError> {
        let ctx = Self::create_carrier_ctx(
            &self.masked_version_bytes,
            &self.dh_pubkey,
            &responder_keys.pk,
        );
        let shared_secret = responder_keys
            .sk()
            .diffie_hellman(&self.dh_pubkey)
            .map_err(KKTError::shared_secret_derivation_failure)?;
        Ok(Carrier::from_secret_slice(
            shared_secret.as_ref(),
            &ctx,
            false,
        ))
    }
}

pub struct KKTRequestEncryptionResult {
    /// Derived carrier used for decrypting this frame and encrypting the response
    pub(crate) carrier: Carrier,

    /// The underlying request that is going to get sent to the remote
    pub(crate) request: KKTRequest,
}

pub struct DecryptedRequestFrame {
    /// Derived carrier used for decrypting this frame and encrypting the response
    pub(crate) carrier: Carrier,

    /// The remote frame sent in the message
    pub(crate) remote_frame: KKTFrame,

    /// The unmasked byte representing the outer protocol version sent by the initiator
    pub(crate) outer_protocol_version: u8,
}

impl DecryptedRequestFrame {
    pub(crate) fn remote_context(&self) -> &KKTContext {
        self.remote_frame.context()
    }
}

pub struct ProcessedKKTRequest {
    pub response: KKTResponse,

    /// The obtained encapsulation key of the remote
    pub remote_encapsulation_key: Option<EncapsulationKey>,

    /// The KEM key requested in the original request
    pub requested_kem: KEM,

    /// The unmasked byte representing the outer protocol version sent by the initiator
    pub outer_protocol_version: u8,

    // Request payload data (Could be empty. Contents are unrelated to current KKT execution).
    pub request_payload: Vec<u8>,
}

pub struct KKTResponse {
    /// Encrypted KKT frame that is going to be sent back to the initiator
    pub encrypted_frame: Vec<u8>,
}

impl KKTResponse {
    // the size of KKTRequest is the plaintext data followed by the frame and the encryption tag
    pub const fn size_excluding_payload(kem: KEM) -> usize {
        // `KKTMode` argument makes no difference for the Responder role
        KKTFrame::size_excluding_payload(KKTRole::Responder, KKTMode::OneWay, kem) + TAG_LEN
    }

    pub fn size(&self) -> usize {
        self.encrypted_frame.len()
    }

    pub fn from_bytes(bytes: Vec<u8>) -> KKTResponse {
        KKTResponse {
            encrypted_frame: bytes,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.encrypted_frame
    }
}

pub struct ProcessedKKTResponse {
    /// The obtained encapsulation key of the remote
    pub encapsulation_key: EncapsulationKey,

    /// Indicates whether responder was able to verify the initiator's kem key,
    pub verified_initiator_kem_key: bool,

    /// Optional response payload (Could be empty. Contents are unrelated to current KKT execution).
    pub response_payload: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_utils::generate_lp_keypair_x25519;
    use nym_test_utils::helpers::deterministic_rng;

    #[test]
    fn peek_returns_the_version_the_initiator_masked() {
        let mut rng = deterministic_rng();
        let initiator = generate_lp_keypair_x25519(&mut rng);
        let responder = generate_lp_keypair_x25519(&mut rng);
        let plaintext = KKTRequestPlaintext::new(initiator.pk, &responder.pk, 7).to_bytes();

        assert_eq!(
            7,
            KKTRequest::peek_outer_protocol_version(&plaintext, &responder.pk, &[1, 7]).unwrap()
        );
        assert!(KKTRequest::peek_outer_protocol_version(&plaintext, &responder.pk, &[1]).is_err());
    }

    #[test]
    fn from_parts_round_trips_into_bytes() {
        let mut rng = deterministic_rng();
        let initiator = generate_lp_keypair_x25519(&mut rng);
        let responder = generate_lp_keypair_x25519(&mut rng);
        let request = KKTRequest {
            plaintext: KKTRequestPlaintext::new(initiator.pk, &responder.pk, 1),
            encrypted_frame: vec![1, 2, 3, 4],
        };
        let bytes = request.into_bytes();
        let (plaintext, frame) = bytes.split_at(KKTRequest::PLAINTEXT_SIZE);

        let rebuilt = KKTRequest::from_parts(plaintext, frame.to_vec()).unwrap();
        assert_eq!(bytes, rebuilt.into_bytes());
    }

    #[test]
    fn from_parts_rejects_a_short_plaintext() {
        assert!(KKTRequest::from_parts(&[0u8; KKTRequest::PLAINTEXT_SIZE - 1], vec![]).is_err());
    }
}
