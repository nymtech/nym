use crate::{
    ciphersuite::{HashFunction, KEM},
    error::KKTError,
};

use classic_mceliece_rust::keypair_boxed;
use libcrux_kem::{Algorithm, key_gen};

use libcrux_sha3;
use nym_crypto::asymmetric::ed25519;
use rand::{CryptoRng, RngCore};
pub fn generate_keypair_ed25519<R>(rng: &mut R, index: Option<u32>) -> ed25519::KeyPair
where
    R: RngCore + CryptoRng,
{
    let mut secret_initiator: [u8; 32] = [0u8; 32];
    rng.fill_bytes(&mut secret_initiator);
    ed25519::KeyPair::from_secret(secret_initiator, index.unwrap_or(0))
}

pub fn generate_keypair_x25519() -> (nym_sphinx::PrivateKey, nym_sphinx::PublicKey) {
    let private_key = nym_sphinx::PrivateKey::random();
    let public_key = nym_sphinx::PublicKey::from(&private_key);
    (private_key, public_key)
}

// (decapsulation_key, encapsulation_key)
pub fn generate_keypair_libcrux<R>(
    rng: &mut R,
    kem: KEM,
) -> Result<(libcrux_kem::PrivateKey, libcrux_kem::PublicKey), KKTError>
where
    R: RngCore + CryptoRng,
{
    match kem {
        KEM::MlKem768 => Ok(key_gen(Algorithm::MlKem768, rng)?),
        KEM::XWing => Ok(key_gen(Algorithm::XWingKemDraft06, rng)?),
        KEM::X25519 => Ok(key_gen(Algorithm::X25519, rng)?),
        _ => Err(KKTError::KEMError {
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
    // this is annoying because mceliece lib uses rand 0.8.5...
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
    let mut hashed_key: Vec<u8> = vec![0u8; hash_length];
    match hash_function {
        HashFunction::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(key_bytes);
            hasher.finalize_xof().fill(&mut hashed_key);
            hasher.reset();
        }
        HashFunction::SHAKE256 => {
            libcrux_sha3::shake256_ema(&mut hashed_key, key_bytes);
        }
        HashFunction::SHAKE128 => {
            libcrux_sha3::shake128_ema(&mut hashed_key, key_bytes);
        }
        HashFunction::SHA256 => {
            libcrux_sha3::sha256_ema(&mut hashed_key, key_bytes);
        }
    }

    hashed_key
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
