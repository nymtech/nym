// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::AckKey;
use crypto::generic_array::typenum::Unsigned;
use crypto::symmetric::stream_cipher::{self, encrypt, iv_from_slice, random_iv, NewStreamCipher};
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

    let iv_size = <AckEncryptionAlgorithm as NewStreamCipher>::NonceSize::to_usize();
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
