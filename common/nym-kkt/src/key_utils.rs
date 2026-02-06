use crate::ciphersuite::HashFunction;
use std::collections::HashMap;

use classic_mceliece_rust::keypair_boxed;

use nym_kkt_ciphersuite::{DEFAULT_HASH_LEN, KeyDigests};
use rand09::{CryptoRng, RngCore};

pub fn generate_keypair_ed25519<R>(
    rng: &mut R,
    index: Option<u32>,
) -> nym_crypto::asymmetric::ed25519::KeyPair
where
    R: RngCore + CryptoRng,
{
    let mut secret_initiator: [u8; 32] = [0u8; 32];
    rng.fill_bytes(&mut secret_initiator);
    nym_crypto::asymmetric::ed25519::KeyPair::from_secret(secret_initiator, index.unwrap_or(0))
}

pub fn generate_keypair_x25519<R>(rng: &mut R) -> nym_crypto::asymmetric::x25519::KeyPair
where
    R: RngCore + CryptoRng,
{
    let mut secret_initiator: [u8; 32] = [0u8; 32];
    rng.fill_bytes(&mut secret_initiator);

    let private_key = nym_crypto::asymmetric::x25519::PrivateKey::from_secret(secret_initiator);
    private_key.into()
}

// (decapsulation_key, encapsulation_key)
pub fn generate_keypair_libcrux<R>(
    rng: &mut R,
    kem: crate::ciphersuite::KEM,
) -> Result<(libcrux_kem::PrivateKey, libcrux_kem::PublicKey), crate::error::KKTError>
where
    R: RngCore + CryptoRng,
{
    match kem {
        crate::ciphersuite::KEM::MlKem768 => {
            Ok(libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, rng)?)
        }
        crate::ciphersuite::KEM::XWing => Ok(libcrux_kem::key_gen(
            libcrux_kem::Algorithm::XWingKemDraft06,
            rng,
        )?),
        crate::ciphersuite::KEM::X25519 => {
            Ok(libcrux_kem::key_gen(libcrux_kem::Algorithm::X25519, rng)?)
        }
        _ => Err(crate::error::KKTError::KEMError {
            info: "Key Generation Error: Unsupported Libcrux Algorithm",
        }),
    }
}
// (decapsulation_key, encapsulation_key)
pub fn generate_keypair_mceliece<'a, R>(
    rng: &mut R,
) -> (
    classic_mceliece_rust::SecretKey<'a>,
    classic_mceliece_rust::PublicKey<'a>,
)
where
    // this is annoying because mceliece lib uses rand09 0.8.5...
    R: RngCore + CryptoRng,
{
    let (encapsulation_key, decapsulation_key) = keypair_boxed(rng);
    (decapsulation_key, encapsulation_key)
}

pub fn hash_key_bytes(
    hash_function: &HashFunction,
    hash_length: usize,
    key_bytes: &[u8],
) -> Vec<u8> {
    hash_function.digest(key_bytes, hash_length)
}

/// attempt to produce digests of the provided key using all known [HashFunction] with a default
/// hash length where variable output is available
pub fn produce_key_digests(key_bytes: &[u8]) -> KeyDigests {
    use strum::IntoEnumIterator;
    let mut digests = HashMap::new();
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
    hash_function: &HashFunction,
    hash_length: usize,
    encapsulation_key: &[u8],
    expected_hash_bytes: &[u8],
) -> bool {
    compare_hashes(
        &hash_encapsulation_key(hash_function, hash_length, encapsulation_key),
        expected_hash_bytes,
    )
}

pub fn validate_key_bytes(
    hash_function: &HashFunction,
    hash_length: usize,
    key_bytes: &[u8],
    expected_hash_bytes: &[u8],
) -> bool {
    compare_hashes(
        &hash_key_bytes(hash_function, hash_length, key_bytes),
        expected_hash_bytes,
    )
}

pub fn hash_encapsulation_key(
    hash_function: &HashFunction,
    hash_length: usize,
    encapsulation_key: &[u8],
) -> Vec<u8> {
    hash_key_bytes(hash_function, hash_length, encapsulation_key)
}
