// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;

use crate::message::{NymMessage, NymMessageError, PaddedMessage, PlainMessage};
use nym_crypto::aes::cipher::{KeyIvInit, StreamCipher};
use nym_crypto::asymmetric::x25519;
use nym_crypto::shared_key::recompute_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_crypto::symmetric::stream_cipher::CipherKey;
use nym_sphinx_anonymous_replies::SurbEncryptionKey;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx_anonymous_replies::requests::SENDER_TAG_SIZE;
use nym_sphinx_chunking::ChunkingError;
use nym_sphinx_chunking::fragment::Fragment;
use nym_sphinx_chunking::reconstruction::MessageReconstructor;
use nym_sphinx_params::{
    PacketEncryptionAlgorithm, PacketHkdfAlgorithm, ReplySurbEncryptionAlgorithm,
};
use thiserror::Error;

/// Error when decoding a `ReconstructedMessage` from bytes.
#[derive(Debug, Error)]
pub enum ReconstructedMessageError {
    #[error("Not enough bytes to decode message: expected at least {expected}, got {received}")]
    TooShort { expected: usize, received: usize },

    #[error("Invalid sender tag flag: expected 0 or 1, got {0}")]
    InvalidSenderTagFlag(u8),
}

/// A message that has been reconstructed from sphinx packets.
///
/// Format:
/// This type uses a simple binary encoding for serialization:
/// - Without sender_tag: `[0][payload...]`
/// - With sender_tag: `[1][16-byte tag][payload...]`
///
/// The first byte indicates whether a sender tag is present (1) or not (0).
/// If present, it is followed by the 16-byte sender tag. The remaining bytes
/// are the message payload.
#[derive(Debug, Clone)]
pub struct ReconstructedMessage {
    /// The actual plaintext message that was received.
    pub message: Vec<u8>,

    /// Optional ephemeral sender tag indicating pseudo-identity of the party who sent us the message
    /// (alongside any reply SURBs)
    pub sender_tag: Option<AnonymousSenderTag>,
}

impl From<ReconstructedMessage> for (Vec<u8>, Option<AnonymousSenderTag>) {
    fn from(msg: ReconstructedMessage) -> Self {
        (msg.message, msg.sender_tag)
    }
}

impl ReconstructedMessage {
    pub fn new(message: Vec<u8>, sender_tag: AnonymousSenderTag) -> Self {
        Self {
            message,
            sender_tag: Some(sender_tag),
        }
    }

    pub fn into_inner(self) -> (Vec<u8>, Option<AnonymousSenderTag>) {
        self.into()
    }

    /// Encodes this message into bytes.
    ///
    /// Format:
    /// - Without sender_tag: `[0][payload...]`
    /// - With sender_tag: `[1][16-byte tag][payload...]`
    pub fn encode(&self) -> Vec<u8> {
        match &self.sender_tag {
            Some(tag) => {
                let mut buf = Vec::with_capacity(1 + SENDER_TAG_SIZE + self.message.len());
                buf.push(1);
                buf.extend_from_slice(&tag.to_bytes());
                buf.extend_from_slice(&self.message);
                buf
            }
            None => {
                let mut buf = Vec::with_capacity(1 + self.message.len());
                buf.push(0);
                buf.extend_from_slice(&self.message);
                buf
            }
        }
    }

    /// Decodes a message from bytes.
    ///
    /// Format:
    /// - Without sender_tag: `[0][payload...]`
    /// - With sender_tag: `[1][16-byte tag][payload...]`
    pub fn decode(bytes: &[u8]) -> Result<Self, ReconstructedMessageError> {
        if bytes.is_empty() {
            return Err(ReconstructedMessageError::TooShort {
                expected: 1,
                received: 0,
            });
        }

        match bytes[0] {
            0 => Ok(ReconstructedMessage {
                message: bytes[1..].to_vec(),
                sender_tag: None,
            }),
            1 => {
                if bytes.len() < 1 + SENDER_TAG_SIZE {
                    return Err(ReconstructedMessageError::TooShort {
                        expected: 1 + SENDER_TAG_SIZE,
                        received: bytes.len(),
                    });
                }
                let tag_bytes: [u8; SENDER_TAG_SIZE] = bytes[1..1 + SENDER_TAG_SIZE]
                    .try_into()
                    .expect("slice length verified above");
                Ok(ReconstructedMessage {
                    message: bytes[1 + SENDER_TAG_SIZE..].to_vec(),
                    sender_tag: Some(AnonymousSenderTag::from_bytes(tag_bytes)),
                })
            }
            flag => Err(ReconstructedMessageError::InvalidSenderTagFlag(flag)),
        }
    }

    /// Returns the encoded size of this message.
    pub fn encoded_size(&self) -> usize {
        1 + self.sender_tag.map_or(0, |_| SENDER_TAG_SIZE) + self.message.len()
    }
}

impl From<PlainMessage> for ReconstructedMessage {
    fn from(message: PlainMessage) -> Self {
        ReconstructedMessage {
            message,
            sender_tag: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum MessageRecoveryError {
    #[error(
        "The received message did not contain enough bytes to recover the ephemeral public key. Got {provided}. required: {required}"
    )]
    NotEnoughBytesForEphemeralKey { provided: usize, required: usize },

    #[error("Recovered remote x25519 public key is invalid - {0}")]
    InvalidRemoteEphemeralKey(#[from] x25519::KeyRecoveryError),

    #[error("The reconstructed message was malformed - {source}")]
    MalformedReconstructedMessage {
        #[source]
        source: NymMessageError,
        used_sets: Vec<i32>,
    },

    #[error("Failed to recover message fragment - {0}")]
    FragmentRecoveryError(#[from] ChunkingError),

    #[error("Failed to recover message fragment - {0}")]
    MessageRecoveryError(#[from] io::Error),
}

pub trait MessageReceiver {
    fn new() -> Self;
    fn reconstructor(&mut self) -> &mut MessageReconstructor;

    fn decrypt_raw_message<C>(
        &self,
        message: &mut [u8],
        key: &CipherKey<C>,
    ) -> Result<(), MessageRecoveryError>
    where
        C: StreamCipher + KeyIvInit;

    fn recover_plaintext_from_reply(
        &self,
        reply_ciphertext: &mut [u8],
        reply_key: SurbEncryptionKey,
    ) -> Result<(), MessageRecoveryError> {
        self.decrypt_raw_message::<ReplySurbEncryptionAlgorithm>(
            reply_ciphertext,
            reply_key.inner(),
        )
    }

    fn recover_plaintext_from_regular_packet<'a>(
        &self,
        local_key: &x25519::PrivateKey,
        raw_enc_frag: &'a mut [u8],
    ) -> Result<&'a mut [u8], MessageRecoveryError> {
        if raw_enc_frag.len() < x25519::PUBLIC_KEY_SIZE {
            return Err(MessageRecoveryError::NotEnoughBytesForEphemeralKey {
                provided: raw_enc_frag.len(),
                required: x25519::PUBLIC_KEY_SIZE,
            });
        }

        // 1. recover remote encryption key
        let remote_key_bytes = &raw_enc_frag[..x25519::PUBLIC_KEY_SIZE];
        let remote_ephemeral_key = x25519::PublicKey::from_bytes(remote_key_bytes)?;

        // 2. recompute shared encryption key
        let encryption_key = recompute_shared_key::<PacketEncryptionAlgorithm, PacketHkdfAlgorithm>(
            &remote_ephemeral_key,
            local_key,
        );

        // 3. decrypt fragment data
        let fragment_ciphertext = &mut raw_enc_frag[x25519::PUBLIC_KEY_SIZE..];

        self.decrypt_raw_message::<PacketEncryptionAlgorithm>(
            fragment_ciphertext,
            &encryption_key,
        )?;
        let fragment_data = fragment_ciphertext;

        Ok(fragment_data)
    }

    fn recover_fragment(&self, frag_data: &[u8]) -> Result<Fragment, MessageRecoveryError> {
        Ok(Fragment::try_from_bytes(frag_data)?)
    }

    fn insert_new_fragment(
        &mut self,
        fragment: Fragment,
    ) -> Result<Option<(NymMessage, Vec<i32>)>, MessageRecoveryError> {
        if let Some((message, used_sets)) = self.reconstructor().insert_new_fragment(fragment) {
            match PaddedMessage::new_reconstructed(message).remove_padding() {
                Ok(message) => Ok(Some((message, used_sets))),
                Err(err) => Err(MessageRecoveryError::MalformedReconstructedMessage {
                    source: err,
                    used_sets,
                }),
            }
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Default)]
pub struct SphinxMessageReceiver {
    /// High level public structure used to buffer all received data [`Fragment`]s and eventually
    /// returning original messages that they encapsulate.
    reconstructor: MessageReconstructor,
}

impl MessageReceiver for SphinxMessageReceiver {
    fn new() -> Self {
        Default::default()
    }

    fn decrypt_raw_message<C>(
        &self,
        message: &mut [u8],
        key: &CipherKey<C>,
    ) -> Result<(), MessageRecoveryError>
    where
        C: StreamCipher + KeyIvInit,
    {
        let zero_iv = stream_cipher::zero_iv::<C>();
        stream_cipher::decrypt_in_place::<C>(key, &zero_iv, message);
        Ok(())
    }

    fn reconstructor(&mut self) -> &mut MessageReconstructor {
        &mut self.reconstructor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_without_sender_tag() {
        let msg = ReconstructedMessage {
            message: b"hello world".to_vec(),
            sender_tag: None,
        };

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0); // flag byte
        assert_eq!(&encoded[1..], b"hello world");

        let decoded = ReconstructedMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.message, b"hello world");
        assert!(decoded.sender_tag.is_none());
    }

    #[test]
    fn encode_decode_with_sender_tag() {
        let tag = AnonymousSenderTag::from_bytes([42u8; SENDER_TAG_SIZE]);
        let msg = ReconstructedMessage {
            message: b"hello world".to_vec(),
            sender_tag: Some(tag),
        };

        let encoded = msg.encode();
        assert_eq!(encoded[0], 1); // flag byte
        assert_eq!(&encoded[1..1 + SENDER_TAG_SIZE], &[42u8; SENDER_TAG_SIZE]);
        assert_eq!(&encoded[1 + SENDER_TAG_SIZE..], b"hello world");

        let decoded = ReconstructedMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.message, b"hello world");
        assert_eq!(
            decoded.sender_tag.unwrap().to_bytes(),
            [42u8; SENDER_TAG_SIZE]
        );
    }

    #[test]
    fn encoded_size_matches() {
        let msg_no_tag = ReconstructedMessage {
            message: b"test".to_vec(),
            sender_tag: None,
        };
        assert_eq!(msg_no_tag.encoded_size(), msg_no_tag.encode().len());

        let msg_with_tag = ReconstructedMessage {
            message: b"test".to_vec(),
            sender_tag: Some(AnonymousSenderTag::from_bytes([0u8; SENDER_TAG_SIZE])),
        };
        assert_eq!(msg_with_tag.encoded_size(), msg_with_tag.encode().len());
    }
}
