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

pub use aes_ctr::stream_cipher::generic_array;
use aes_ctr::{
    stream_cipher::{
        generic_array::{
            typenum::{Unsigned, U16},
            GenericArray,
        },
        NewStreamCipher, SyncStreamCipher,
    },
    Aes128Ctr,
};
use rand::{CryptoRng, RngCore};

// the 'U16' type is taken directly from the `Ctr128` for consistency sake
pub type Aes128KeySize = U16;
pub type Aes128NonceSize = U16;

pub type Aes128Key = GenericArray<u8, Aes128KeySize>;
pub type Aes128IV = GenericArray<u8, Aes128NonceSize>;

pub fn generate_key<R: RngCore + CryptoRng>(rng: &mut R) -> Aes128Key {
    let mut ack_key = GenericArray::default();
    rng.fill_bytes(&mut ack_key);
    ack_key
}

pub fn random_iv<R: RngCore + CryptoRng>(rng: &mut R) -> Aes128IV {
    let mut iv = GenericArray::default();
    rng.fill_bytes(&mut iv);
    iv
}

pub fn zero_iv() -> Aes128IV {
    GenericArray::default()
}

pub fn iv_from_slice(b: &[u8]) -> &Aes128IV {
    if b.len() != Aes128NonceSize::to_usize() {
        // `from_slice` would have caused a panic about this issue anyway.
        // Now we at least have slightly more information
        panic!(
            "Tried to convert {} bytes to IV. Expected {}",
            b.len(),
            Aes128NonceSize::to_usize()
        )
    }
    GenericArray::from_slice(b)
}

fn apply_aes_ctr(key: &Aes128Key, iv: &Aes128IV, mut data: &mut [u8]) {
    let mut cipher = Aes128Ctr::new(key, iv);
    cipher.apply_keystream(&mut data)
}

pub fn encrypt(key: &Aes128Key, iv: &Aes128IV, data: &[u8]) -> Vec<u8> {
    let mut ciphertext = data.to_vec();
    apply_aes_ctr(key, iv, &mut ciphertext);
    ciphertext
}

pub fn encrypt_in_place(key: &Aes128Key, iv: &Aes128IV, data: &mut [u8]) {
    apply_aes_ctr(key, iv, data)
}

pub fn decrypt(key: &Aes128Key, iv: &Aes128IV, ciphertext: &[u8]) -> Vec<u8> {
    let mut data = ciphertext.to_vec();
    apply_aes_ctr(key, iv, &mut data);
    data
}

pub fn decrypt_in_place(key: &Aes128Key, iv: &Aes128IV, ciphertext: &mut [u8]) {
    apply_aes_ctr(key, iv, ciphertext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn zero_iv_is_actually_zero() {
        let iv = zero_iv();
        for b in iv {
            assert_eq!(b, 0);
        }
    }

    #[test]
    fn decryption_is_reciprocal_to_encryption() {
        let mut rng = OsRng;

        let arr_input = [42; 200];
        let vec_input = vec![123, 200];

        let key1 = generate_key(&mut rng);
        let key2 = generate_key(&mut rng);

        let iv1 = random_iv(&mut rng);
        let iv2 = random_iv(&mut rng);

        let ciphertext1 = encrypt(&key1, &iv1, &arr_input);
        let ciphertext2 = encrypt(&key2, &iv2, &vec_input);

        assert_eq!(arr_input.to_vec(), decrypt(&key1, &iv1, &ciphertext1));
        assert_eq!(vec_input, decrypt(&key2, &iv2, &ciphertext2));
    }

    #[test]
    fn in_place_variants_work_same_way() {
        let mut rng = OsRng;

        let mut data = [42; 200];
        let original_data = data.clone();

        let key = generate_key(&mut rng);
        let iv = random_iv(&mut rng);

        encrypt_in_place(&key, &iv, &mut data);
        assert_eq!(data.to_vec(), encrypt(&key, &iv, &original_data));

        decrypt_in_place(&key, &iv, &mut data);
        assert_eq!(data.to_vec(), original_data.to_vec());
    }
}
