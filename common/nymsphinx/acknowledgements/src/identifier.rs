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

use crate::key::AckAes128Key;
use crypto::symmetric::aes_ctr::generic_array::typenum::Unsigned;
use crypto::symmetric::aes_ctr::{iv_from_slice, Aes128IV, Aes128NonceSize};
use nymsphinx_params::packet_sizes::PacketSize;
use rand::{CryptoRng, RngCore};

type AckAes128IV = Aes128IV;

fn random_iv<R: RngCore + CryptoRng>(rng: &mut R) -> AckAes128IV {
    crypto::symmetric::aes_ctr::random_iv(rng)
}

pub fn prepare_identifier<R: RngCore + CryptoRng>(
    rng: &mut R,
    key: &AckAes128Key,
    marshaled_id: [u8; 5],
) -> Vec<u8> {
    let iv = random_iv(rng);
    let id_ciphertext = crypto::symmetric::aes_ctr::encrypt(key, &iv, &marshaled_id);

    // IV || ID_CIPHERTEXT
    iv.into_iter().chain(id_ciphertext.into_iter()).collect()
}

pub fn recover_identifier(key: &AckAes128Key, iv_id_ciphertext: &[u8]) -> Option<[u8; 5]> {
    // first few bytes are expected to be the concatenated IV. It must be followed by at least 1 more
    // byte that we wish to recover, but it can be no longer from what we can physically store inside
    // an ack
    if iv_id_ciphertext.len() != PacketSize::ACKPacket.plaintext_size() {
        return None;
    }

    let iv = iv_from_slice(&iv_id_ciphertext[..Aes128NonceSize::to_usize()]);
    let id = crypto::symmetric::aes_ctr::decrypt(
        key,
        iv,
        &iv_id_ciphertext[Aes128NonceSize::to_usize()..],
    );

    let mut id_arr = [0u8; 5];
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
        let key = AckAes128Key::new(&mut rng);

        let id = [1, 2, 3, 4, 5];
        let iv_ciphertext = prepare_identifier(&mut rng, &key, id);
        assert_eq!(
            id.to_vec(),
            recover_identifier(&key, &iv_ciphertext).unwrap()
        );
    }
}
