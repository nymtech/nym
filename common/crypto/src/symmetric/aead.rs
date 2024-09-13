// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use aead::{Aead, AeadCore, AeadInPlace, Buffer, KeyInit, Payload};

#[cfg(feature = "rand")]
use rand::{CryptoRng, RngCore};

pub use aead::{Error as AeadError, Key as AeadKey, KeySizeUser, Nonce, Tag};

#[cfg(feature = "rand")]
pub fn generate_key<A, R>(rng: &mut R) -> AeadKey<A>
where
    A: KeyInit,
    R: RngCore + CryptoRng,
{
    let mut key = AeadKey::<A>::default();
    rng.fill_bytes(&mut key);
    key
}

#[cfg(feature = "rand")]
pub fn random_nonce<A, R>(rng: &mut R) -> Nonce<A>
where
    A: AeadCore,
    R: RngCore + CryptoRng,
{
    <A as AeadCore>::generate_nonce(rng)
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
