// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use aead::{Aead, AeadCore, AeadInPlace, Buffer, KeyInit, Payload};
use generic_array::typenum::Unsigned;

#[cfg(feature = "rand")]
use rand::CryptoRng;

pub use aead::{Error as AeadError, Key as AeadKey, KeySizeUser, Nonce, Tag};

#[cfg(feature = "rand")]
pub fn generate_key<A, R>(rng: &mut R) -> AeadKey<A>
where
    A: KeyInit,
    R: CryptoRng,
{
    let mut key = AeadKey::<A>::default();
    rng.fill_bytes(&mut key);
    key
}

// `AeadCore::generate_nonce` (from the `aead` crate) still requires a rand_core 0.6 rng, since
// the RustCrypto AEAD crates haven't followed the dalek crates onto rand_core 0.10 yet. Its
// default implementation is just filling a zeroed nonce, so we reproduce that directly instead
// of depending on the trait method's older rng bound.
#[cfg(feature = "rand")]
pub fn random_nonce<A, R>(rng: &mut R) -> Nonce<A>
where
    A: AeadCore,
    Nonce<A>: Default,
    R: CryptoRng,
{
    let mut nonce = Nonce::<A>::default();
    rng.fill_bytes(&mut nonce);
    nonce
}

pub fn nonce_size<A>() -> usize
where
    A: AeadCore,
{
    <<A as AeadCore>::NonceSize>::to_usize()
}

pub fn tag_size<A>() -> usize
where
    A: AeadCore,
{
    <<A as AeadCore>::TagSize>::to_usize()
}

#[inline]
pub fn encrypt<'msg, 'aad, A>(
    key: &AeadKey<A>,
    nonce: &Nonce<A>,
    plaintext: impl Into<Payload<'msg, 'aad>>,
) -> Result<Vec<u8>, AeadError>
where
    A: Aead + KeyInit,
{
    let cipher = A::new(key);
    cipher.encrypt(nonce, plaintext)
}

#[inline]
pub fn decrypt<'msg, 'aad, A>(
    key: &AeadKey<A>,
    nonce: &Nonce<A>,
    ciphertext: impl Into<Payload<'msg, 'aad>>,
) -> Result<Vec<u8>, AeadError>
where
    A: Aead + KeyInit,
{
    let cipher = A::new(key);
    cipher.decrypt(nonce, ciphertext)
}

#[inline]
pub fn encrypt_in_place<A>(
    key: &AeadKey<A>,
    nonce: &Nonce<A>,
    associated_data: &[u8],
    buffer: &mut dyn Buffer,
) -> Result<(), AeadError>
where
    A: AeadInPlace + KeyInit,
{
    let cipher = A::new(key);
    cipher.encrypt_in_place(nonce, associated_data, buffer)
}

#[inline]
pub fn decrypt_in_place<A>(
    key: &AeadKey<A>,
    nonce: &Nonce<A>,
    associated_data: &[u8],
    buffer: &mut dyn Buffer,
) -> Result<(), AeadError>
where
    A: AeadInPlace + KeyInit,
{
    let cipher = A::new(key);
    cipher.decrypt_in_place(nonce, associated_data, buffer)
}
