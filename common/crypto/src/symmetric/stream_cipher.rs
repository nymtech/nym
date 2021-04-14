// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cipher::stream::{Nonce, StreamCipher, SyncStreamCipher};
use generic_array::{typenum::Unsigned, GenericArray};
use rand::{CryptoRng, RngCore};

// re-export this for ease of use
pub use cipher::stream::{Key, NewStreamCipher};

// TODO: note that this is not the most secure approach here
// we are not using nonces properly but instead "kinda" thinking of them as IVs.
// Nonce require, as the name suggest, being only seen once. Ever.
// While what we are doing here, i.e. generating a pseudo-random IV,
// means that for, for example, 128-bit security, after generating 2^64 IVs
// we are going to have 50% chance of collision. But perhaps that's fine?
// TODO2: ask @AP if what I wrote here even makes sense in the context of what we're doing.

// I think 'IV' looks better than 'Iv', feel free to change that.
#[allow(clippy::upper_case_acronyms)]
pub type IV<C> = Nonce<C>;

pub fn generate_key<C, R>(rng: &mut R) -> Key<C>
where
    C: NewStreamCipher,
    R: RngCore + CryptoRng,
{
    let mut key = GenericArray::default();
    rng.fill_bytes(&mut key);
    key
}

pub fn random_iv<C, R>(rng: &mut R) -> IV<C>
where
    C: NewStreamCipher,
    R: RngCore + CryptoRng,
{
    let mut iv = GenericArray::default();
    rng.fill_bytes(&mut iv);
    iv
}

pub fn zero_iv<C>() -> IV<C>
where
    C: NewStreamCipher,
{
    GenericArray::default()
}

pub fn iv_from_slice<C>(b: &[u8]) -> &IV<C>
where
    C: NewStreamCipher,
{
    if b.len() != C::NonceSize::to_usize() {
        // `from_slice` would have caused a panic about this issue anyway.
        // Now we at least have slightly more information
        panic!(
            "Tried to convert {} bytes to IV. Expected {}",
            b.len(),
            C::NonceSize::to_usize()
        )
    }
    GenericArray::from_slice(b)
}

// TODO: there's really no way to use more parts of the keystream if it was required at some point.
// However, do we really expect to ever need it?

pub fn encrypt<C>(key: &Key<C>, iv: &IV<C>, data: &[u8]) -> Vec<u8>
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut ciphertext = data.to_vec();
    encrypt_in_place::<C>(key, iv, &mut ciphertext);
    ciphertext
}

pub fn encrypt_in_place<C>(key: &Key<C>, iv: &IV<C>, data: &mut [u8])
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut cipher = C::new(key, iv);
    cipher.encrypt(data)
}

pub fn decrypt<C>(key: &Key<C>, iv: &IV<C>, ciphertext: &[u8]) -> Vec<u8>
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut data = ciphertext.to_vec();
    decrypt_in_place::<C>(key, iv, &mut data);
    data
}

pub fn decrypt_in_place<C>(key: &Key<C>, iv: &IV<C>, data: &mut [u8])
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut cipher = C::new(key, iv);
    cipher.decrypt(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[cfg(test)]
    mod aes_ctr128 {
        use super::*;
        use aes_ctr::Aes128Ctr;

        #[test]
        fn zero_iv_is_actually_zero() {
            let iv = zero_iv::<Aes128Ctr>();
            for b in iv {
                assert_eq!(b, 0);
            }
        }

        #[test]
        fn decryption_is_reciprocal_to_encryption() {
            let mut rng = OsRng;

            let arr_input = [42; 200];
            let vec_input = vec![123, 200];

            let key1 = generate_key::<Aes128Ctr, _>(&mut rng);
            let key2 = generate_key::<Aes128Ctr, _>(&mut rng);

            let iv1 = random_iv::<Aes128Ctr, _>(&mut rng);
            let iv2 = random_iv::<Aes128Ctr, _>(&mut rng);

            let ciphertext1 = encrypt::<Aes128Ctr>(&key1, &iv1, &arr_input);
            let ciphertext2 = encrypt::<Aes128Ctr>(&key2, &iv2, &vec_input);

            assert_eq!(
                arr_input.to_vec(),
                decrypt::<Aes128Ctr>(&key1, &iv1, &ciphertext1)
            );
            assert_eq!(vec_input, decrypt::<Aes128Ctr>(&key2, &iv2, &ciphertext2));
        }

        #[test]
        fn in_place_variants_work_same_way() {
            let mut rng = OsRng;

            let mut data = [42; 200];
            let original_data = data;

            let key = generate_key::<Aes128Ctr, _>(&mut rng);
            let iv = random_iv::<Aes128Ctr, _>(&mut rng);

            encrypt_in_place::<Aes128Ctr>(&key, &iv, &mut data);
            assert_eq!(
                data.to_vec(),
                encrypt::<Aes128Ctr>(&key, &iv, &original_data)
            );

            decrypt_in_place::<Aes128Ctr>(&key, &iv, &mut data);
            assert_eq!(data.to_vec(), original_data.to_vec());
        }
    }
}
