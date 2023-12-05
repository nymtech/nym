// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::message::{NymMessage, NymMessageError, PaddedMessage, PlainMessage};
use nym_crypto::aes::cipher::{KeyIvInit, StreamCipher};
use nym_crypto::asymmetric::encryption;
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
    DEFAULT_NUM_MIX_HOPS,
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
    InvalidRemoteEphemeralKey(#[from] encryption::KeyRecoveryError),

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
    fn num_mix_hops(&self) -> u8;

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
        local_key: &encryption::PrivateKey,
        raw_enc_frag: &'a mut [u8],
    ) -> Result<&'a mut [u8], MessageRecoveryError> {
        if raw_enc_frag.len() < encryption::PUBLIC_KEY_SIZE {
            return Err(MessageRecoveryError::NotEnoughBytesForEphemeralKey {
                provided: raw_enc_frag.len(),
                required: encryption::PUBLIC_KEY_SIZE,
            });
        }

        // 1. recover remote encryption key
        let remote_key_bytes = &raw_enc_frag[..encryption::PUBLIC_KEY_SIZE];
        let remote_ephemeral_key = encryption::PublicKey::from_bytes(remote_key_bytes)?;

        // 2. recompute shared encryption key
        let encryption_key = recompute_shared_key::<PacketEncryptionAlgorithm, PacketHkdfAlgorithm>(
            &remote_ephemeral_key,
            local_key,
        );

        // 3. decrypt fragment data
        let fragment_ciphertext = &mut raw_enc_frag[encryption::PUBLIC_KEY_SIZE..];

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
            match PaddedMessage::new_reconstructed(message).remove_padding(self.num_mix_hops()) {
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

#[derive(Clone)]
pub struct SphinxMessageReceiver {
    /// High level public structure used to buffer all received data [`Fragment`]s and eventually
    /// returning original messages that they encapsulate.
    reconstructor: MessageReconstructor,

    /// Number of mix hops each packet ('real' message, ack, reply) is expected to take.
    /// Note that it does not include gateway hops.
    num_mix_hops: u8,
}

impl SphinxMessageReceiver {
    /// Allows setting non-default number of expected mix hops in the network.
    // IMPORTANT NOTE: this is among others used to deserialize SURBs. Meaning that this is a
    // global setting and currently always set to the default value. The implication is that it is
    // not currently possible to have different number of hops for different SURB messages. So,
    // don't try to use <3 mix hops for SURBs until this is refactored.
    #[must_use]
    pub fn with_mix_hops(mut self, hops: u8) -> Self {
        self.num_mix_hops = hops;
        self
    }
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

    fn num_mix_hops(&self) -> u8 {
        self.num_mix_hops
    }
}

impl Default for SphinxMessageReceiver {
    fn default() -> Self {
        SphinxMessageReceiver {
            reconstructor: Default::default(),
            num_mix_hops: DEFAULT_NUM_MIX_HOPS,
        }
    }
}

#[cfg(test)]
mod message_receiver {
    use super::*;
    use nym_crypto::asymmetric::identity;
    use nym_mixnet_contract_common::Layer;
    use nym_topology::{gateway, mix, NymTopology};
    use std::collections::BTreeMap;

    // TODO: is it somehow maybe possible to move it to `topology` and have if conditionally
    // available to other modules?
    /// Returns a hardcoded, valid instance of [`NymTopology`] that is to be used in
    /// tests requiring instance of topology.
    #[allow(dead_code)]
    fn topology_fixture() -> NymTopology {
        let mut mixes = BTreeMap::new();
        mixes.insert(
            1,
            vec![mix::Node {
                mix_id: 123,
                owner: "foomp1".to_string(),
                host: "10.20.30.40".parse().unwrap(),
                mix_host: "10.20.30.40:1789".parse().unwrap(),
                identity_key: identity::PublicKey::from_base58_string(
                    "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "B3GzG62aXAZNg14RoMCp3BhELNBrySLr2JqrwyfYFzRc",
                )
                .unwrap(),
                layer: Layer::One,
                version: "0.8.0-dev".into(),
            }],
        );

        mixes.insert(
            2,
            vec![mix::Node {
                mix_id: 234,
                owner: "foomp2".to_string(),
                host: "11.21.31.41".parse().unwrap(),
                mix_host: "11.21.31.41:1789".parse().unwrap(),
                identity_key: identity::PublicKey::from_base58_string(
                    "D6YaMzLSY7mANtSQRKXsmMZpqgqiVkeiagKM4V4oFPFr",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "5Z1VqYwM2xeKxd8H7fJpGWasNiDFijYBAee7MErkZ5QT",
                )
                .unwrap(),
                layer: Layer::Two,
                version: "0.8.0-dev".into(),
            }],
        );

        mixes.insert(
            3,
            vec![mix::Node {
                mix_id: 456,
                owner: "foomp3".to_string(),
                host: "12.22.32.42".parse().unwrap(),
                mix_host: "12.22.32.42:1789".parse().unwrap(),
                identity_key: identity::PublicKey::from_base58_string(
                    "GkWDysw4AjESv1KiAiVn7JzzCMJeksxNSXVfr1PpX8wD",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "9EyjhCggr2QEA2nakR88YHmXgpy92DWxoe2draDRkYof",
                )
                .unwrap(),
                layer: Layer::Three,
                version: "0.8.0-dev".into(),
            }],
        );

        NymTopology::new(
            // currently coco_nodes don't really exist so this is still to be determined
            mixes,
            vec![gateway::Node {
                owner: "foomp4".to_string(),
                host: "1.2.3.4".parse().unwrap(),
                mix_host: "1.2.3.4:1789".parse().unwrap(),
                clients_ws_port: 9000,
                clients_wss_port: None,
                identity_key: identity::PublicKey::from_base58_string(
                    "FioFa8nMmPpQnYi7JyojoTuwGLeyNS8BF4ChPr29zUML",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "EB42xvMFMD5rUCstE2CDazgQQJ22zLv8SPm1Luxni44c",
                )
                .unwrap(),
                version: "0.8.0-dev".into(),
            }],
        )
    }
}
