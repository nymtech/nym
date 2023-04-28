// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::aes::cipher::{KeyIvInit, StreamCipher};
use nym_crypto::asymmetric::encryption;
use nym_crypto::shared_key::new_ephemeral_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_crypto::symmetric::stream_cipher::CipherKey;
use nym_sphinx_acknowledgements::surb_ack::{SurbAck, SurbAckRecoveryError};
use nym_sphinx_anonymous_replies::SurbEncryptionKey;
use nym_sphinx_chunking::fragment::Fragment;
use nym_sphinx_params::{
    PacketEncryptionAlgorithm, PacketHkdfAlgorithm, ReplySurbEncryptionAlgorithm,
};
use rand::{CryptoRng, RngCore};

pub struct NymPayloadBuilder {
    fragment: Fragment,
    surb_ack: SurbAck,
}

impl NymPayloadBuilder {
    pub fn new(fragment: Fragment, surb_ack: SurbAck) -> Self {
        NymPayloadBuilder { fragment, surb_ack }
    }

    fn build<C>(
        self,
        packet_encryption_key: &CipherKey<C>,
        variant_data: impl IntoIterator<Item = u8>,
    ) -> Result<NymPayload, SurbAckRecoveryError>
    where
        C: StreamCipher + KeyIvInit,
    {
        let (_, surb_ack_bytes) = self.surb_ack.prepare_for_sending()?;

        let mut fragment_data = self.fragment.into_bytes();
        stream_cipher::encrypt_in_place::<C>(
            packet_encryption_key,
            &stream_cipher::zero_iv::<C>(),
            &mut fragment_data,
        );

        // combines all the data as follows:
        // SURB_ACK || VARIANT_SPECIFIC_DATA || CHUNK_DATA
        // where variant-specific data is as follows:
        // for replies it would be the digest of the encryption key used
        // for 'regular' messages it would be the public component used in DH later used in the KDF
        Ok(NymPayload(
            surb_ack_bytes
                .into_iter()
                .chain(variant_data.into_iter())
                .chain(fragment_data.into_iter())
                .collect(),
        ))
    }

    pub fn build_reply(
        self,
        packet_encryption_key: &SurbEncryptionKey,
    ) -> Result<NymPayload, SurbAckRecoveryError> {
        let key_digest = packet_encryption_key.compute_digest();
        self.build::<ReplySurbEncryptionAlgorithm>(
            packet_encryption_key.inner(),
            key_digest.into_iter(),
        )
    }

    pub fn build_regular<R>(
        self,
        rng: &mut R,
        recipient_encryption_key: &encryption::PublicKey,
    ) -> Result<NymPayload, SurbAckRecoveryError>
    where
        R: RngCore + CryptoRng,
    {
        // create keys for 'payload' encryption
        let (ephemeral_keypair, shared_key) = new_ephemeral_shared_key::<
            PacketEncryptionAlgorithm,
            PacketHkdfAlgorithm,
            _,
        >(rng, recipient_encryption_key);

        self.build::<PacketEncryptionAlgorithm>(
            &shared_key,
            ephemeral_keypair.public_key().to_bytes(),
        )
    }
}

// the actual byte data that will be put into the sphinx packet paylaod.
// no more transformations are going to happen to it
// TODO: use that fact for some better compile time assertions
pub struct NymPayload(Vec<u8>);

impl AsRef<[u8]> for NymPayload {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
