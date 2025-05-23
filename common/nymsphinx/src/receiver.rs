// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::message::{NymMessage, NymMessageError, PaddedMessage, PlainMessage};
use nym_crypto::aes::cipher::{KeyIvInit, StreamCipher};
use nym_crypto::asymmetric::x25519;
use nym_crypto::shared_key::recompute_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_crypto::symmetric::stream_cipher::CipherKey;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx_anonymous_replies::SurbEncryptionKey;
use nym_sphinx_chunking::fragment::Fragment;
use nym_sphinx_chunking::reconstruction::MessageReconstructor;
use nym_sphinx_chunking::ChunkingError;
use nym_sphinx_params::{
    PacketEncryptionAlgorithm, PacketHkdfAlgorithm, ReplySurbEncryptionAlgorithm,
};
use thiserror::Error;

// TODO: should this live in this file?
#[derive(Debug)]
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
    #[error("The received message did not contain enough bytes to recover the ephemeral public key. Got {provided}. required: {required}")]
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
