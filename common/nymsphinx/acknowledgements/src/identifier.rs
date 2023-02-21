// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::AckKey;
use crypto::symmetric::stream_cipher::{self, encrypt, iv_from_slice, random_iv, IvSizeUser};
use nymsphinx_params::{
    packet_sizes::PacketSize, AckEncryptionAlgorithm, SerializedFragmentIdentifier, FRAG_ID_LEN,
};
use rand::{CryptoRng, RngCore};

// TODO: should those functions even exist in this file?

pub fn prepare_identifier<R: RngCore + CryptoRng>(
    rng: &mut R,
    key: &AckKey,
    serialized_id: SerializedFragmentIdentifier,
) -> Vec<u8> {
    let iv = random_iv::<AckEncryptionAlgorithm, _>(rng);
    let id_ciphertext = encrypt::<AckEncryptionAlgorithm>(key.inner(), &iv, &serialized_id);

    // IV || ID_CIPHERTEXT
    iv.into_iter().chain(id_ciphertext.into_iter()).collect()
}

pub fn recover_identifier(
    key: &AckKey,
    iv_id_ciphertext: &[u8],
) -> Option<SerializedFragmentIdentifier> {
    // The content of an 'ACK' packet consists of AckEncryptionAlgorithm::IV followed by
    // serialized FragmentIdentifier
    if iv_id_ciphertext.len() != PacketSize::AckPacket.plaintext_size() {
        return None;
    }

    let iv_size = AckEncryptionAlgorithm::iv_size();
    let iv = iv_from_slice::<AckEncryptionAlgorithm>(&iv_id_ciphertext[..iv_size]);

    let id = stream_cipher::decrypt::<AckEncryptionAlgorithm>(
        key.inner(),
        iv,
        &iv_id_ciphertext[iv_size..],
    );

    let mut id_arr = [0u8; FRAG_ID_LEN];
    id_arr.copy_from_slice(&id);
    Some(id_arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn id_is_recoverable() {
        let mut rng = OsRng;
        let key = AckKey::new(&mut rng);

        let id = [1, 2, 3, 4, 5];
        let iv_ciphertext = prepare_identifier(&mut rng, &key, id);
        assert_eq!(
            id.to_vec(),
            recover_identifier(&key, &iv_ciphertext).unwrap()
        );
    }
}
