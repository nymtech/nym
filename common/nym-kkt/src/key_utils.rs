// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_ml_kem::mlkem768::MlKem768KeyPair;
use libcrux_psq::handshake::types::DHKeyPair;
use nym_kkt_ciphersuite::{DEFAULT_HASH_LEN, HashFunction, KEMKeyDigests};
use rand09::{CryptoRng, RngCore};
use std::collections::BTreeMap;

pub fn generate_lp_keypair_x25519<R>(rng: &mut R) -> DHKeyPair
where
    R: RngCore + CryptoRng,
{
    DHKeyPair::new(rng)
}

pub fn generate_keypair_mlkem<R>(rng: &mut R) -> MlKem768KeyPair
where
    R: RngCore + CryptoRng,
{
    libcrux_ml_kem::mlkem768::rand::generate_key_pair(rng)
}

pub fn generate_keypair_mceliece<R>(rng: &mut R) -> libcrux_psq::classic_mceliece::KeyPair
where
    R: RngCore + CryptoRng,
{
    libcrux_psq::classic_mceliece::KeyPair::generate_key_pair(rng)
}

pub fn hash_key_bytes(
    hash_function: HashFunction,
    hash_length: usize,
    key_bytes: &[u8],
) -> Vec<u8> {
    hash_function.digest(key_bytes, hash_length)
}

/// attempt to produce digests of the provided key using all known [HashFunction] with a default
/// hash length where variable output is available
pub fn produce_key_digests(key_bytes: &[u8]) -> KEMKeyDigests {
    use strum::IntoEnumIterator;
    let mut digests = BTreeMap::new();
    for hash in HashFunction::iter() {
        digests.insert(hash, hash.digest(key_bytes, DEFAULT_HASH_LEN));
    }
    digests
}

/// This does NOT run in constant time.
// It's fine for KKT since we are comparing hashes.
fn compare_hashes(a: &[u8], b: &[u8]) -> bool {
    a == b
}

pub fn validate_encapsulation_key(
    hash_function: HashFunction,
    hash_length: usize,
    encapsulation_key: &[u8],
    expected_hash_bytes: &[u8],
) -> bool {
    compare_hashes(
        &hash_encapsulation_key(hash_function, hash_length, encapsulation_key),
        expected_hash_bytes,
    )
}

pub fn hash_encapsulation_key(
    hash_function: HashFunction,
    hash_length: usize,
    encapsulation_key: &[u8],
) -> Vec<u8> {
    hash_key_bytes(hash_function, hash_length, encapsulation_key)
}
