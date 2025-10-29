use core::hash;

use blake3::{Hash, Hasher};
use curve25519_dalek::digest::DynDigest;
use libcrux_psq::traits::Ciphertext;
use nym_crypto::symmetric::aead::{AeadKey, Nonce};
use nym_crypto::{
    aes::Aes256,
    asymmetric::x25519::{self, PrivateKey, PublicKey},
    generic_array::GenericArray,
    Aes256GcmSiv,
};
// use rand::{CryptoRng, RngCore};
use zeroize::Zeroize;

use nym_crypto::aes::cipher::crypto_common::rand_core::{CryptoRng, RngCore};

use crate::error::KKTError;

fn generate_round_trip_symmetric_key<R>(
    rng: &mut R,
    remote_public_key: &PublicKey,
) -> ([u8; 64], [u8; 32])
where
    R: CryptoRng + RngCore,
{
    let mut s = x25519::PrivateKey::new(rng);
    let gs = s.public_key();

    let mut gbs = s.diffie_hellman(remote_public_key);
    s.zeroize();

    let mut message: [u8; 64] = [0u8; 64];
    message[0..32].clone_from_slice(gs.as_bytes());

    let mut hasher = Hasher::new();

    hasher.update(&gbs);
    gbs.zeroize();
    let key: [u8; 32] = hasher.finalize().as_bytes().to_owned();

    hasher.update(remote_public_key.as_bytes());
    hasher.update(gs.as_bytes());

    hasher.finalize_into_reset(&mut message[32..64]);

    (message, key)
}

fn extract_shared_secret(b: &PrivateKey, message: &[u8; 64]) -> Result<[u8; 32], KKTError> {
    let gs = PublicKey::from_bytes(&message[0..32])?;

    let mut gsb = b.diffie_hellman(&gs);

    let mut hasher = Hasher::new();
    hasher.update(&gsb);
    gsb.zeroize();
    let key: [u8; 32] = hasher.finalize().as_bytes().to_owned();

    hasher.update(b.public_key().as_bytes());
    hasher.update(gs.as_bytes());

    // This runs in constant time
    if hasher.finalize() == message[32..64] {
        Ok(key)
    } else {
        Err(KKTError::X25519Error {
            info: format!("Symmetric Key Hash Validation Error"),
        })
    }
}

fn encrypt(mut key: [u8; 32], message: &[u8]) -> Result<Vec<u8>, KKTError> {
    // The empty nonce is fine since we use the key once.
    let nonce = Nonce::<Aes256GcmSiv>::from_slice(&[]);

    let ciphertext =
        nym_crypto::symmetric::aead::encrypt::<Aes256GcmSiv>(&key.into(), nonce, message)?;

    key.zeroize();

    Ok(ciphertext)
}

fn decrypt(key: [u8; 32], ciphertext: Vec<u8>) -> Vec<u8> {
    // The empty nonce is fine since we use the key once.
    let nonce = Nonce::<Aes256>::from_slice(&[]);

    let ciphertext =
        nym_crypto::symmetric::aead::encrypt::<Aes256GcmSiv>(&key.into(), nonce, message)?;

    key.zeroize();

    Ok(ciphertext)
}
