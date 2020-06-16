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

use aes_ctr::{
    stream_cipher::{
        generic_array::{
            typenum::{marker_traits::Unsigned, U16},
            GenericArray,
        },
        NewStreamCipher, SyncStreamCipher,
    },
    Aes128Ctr,
};
use nymsphinx_params::packet_sizes::PacketSize;
use rand::{CryptoRng, RngCore};

// the 'U16' type is taken directly from the `Ctr128` for consistency sake
pub type Aes128KeySize = U16;
pub type Aes128NonceSize = U16;

pub type AckAes128Key = GenericArray<u8, Aes128KeySize>;
type AckAes128IV = GenericArray<u8, Aes128NonceSize>;

pub fn generate_key<R: RngCore + CryptoRng>(rng: &mut R) -> AckAes128Key {
    let mut ack_key = GenericArray::default();
    rng.fill_bytes(&mut ack_key);
    ack_key
}

fn random_iv<R: RngCore + CryptoRng>(rng: &mut R) -> AckAes128IV {
    let mut iv = GenericArray::default();
    rng.fill_bytes(&mut iv);
    iv
}

pub fn prepare_identifier<R: RngCore + CryptoRng>(
    rng: &mut R,
    key: &AckAes128Key,
    marshaled_id: &[u8],
) -> Vec<u8> {
    // TODO: should we have some length checks on the id?

    let iv = random_iv(rng);
    let mut cipher = Aes128Ctr::new(key, &iv);
    let mut output = marshaled_id.to_vec();

    cipher.apply_keystream(&mut output);

    iv.into_iter().chain(output.into_iter()).collect()
}

pub fn recover_identifier(key: &AckAes128Key, iv_ciphertext: &[u8]) -> Option<Vec<u8>> {
    // first few bytes are expected to be the concatenated IV. It must be followed by at least 1 more
    // byte that we wish to recover, but it can be no longer from what we can physically store inside
    // an ack
    if iv_ciphertext.len() <= Aes128NonceSize::to_usize()
        || iv_ciphertext.len() > PacketSize::ACKPacket.plaintext_size()
    {
        return None;
    }

    let iv = GenericArray::from_slice(&iv_ciphertext[..Aes128NonceSize::to_usize()]);
    let mut cipher = Aes128Ctr::new(key, &iv);
    let mut output = iv_ciphertext[Aes128NonceSize::to_usize()..].to_vec();
    cipher.apply_keystream(&mut output);

    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn id_is_recoverable() {
        let mut rng = OsRng;
        let key = generate_key(&mut rng);

        let id1 = vec![42]; // single byte case
        let id2 = vec![1, 2, 3, 4, 5]; // 5byte we expect to use
        let id3 = vec![42; 8]; // some reasonable upper bound id size we could use later on

        let iv_ciphertext1 = prepare_identifier(&mut rng, &key, &id1);
        let iv_ciphertext2 = prepare_identifier(&mut rng, &key, &id2);
        let iv_ciphertext3 = prepare_identifier(&mut rng, &key, &id3);

        assert_eq!(id1, recover_identifier(&key, &iv_ciphertext1).unwrap());
        assert_eq!(id2, recover_identifier(&key, &iv_ciphertext2).unwrap());
        assert_eq!(id3, recover_identifier(&key, &iv_ciphertext3).unwrap());
    }
}
