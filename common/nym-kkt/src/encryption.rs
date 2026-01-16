// Copyright 2025-2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::kkt::KKT_INITIAL_FRAME_AAD;
use crate::{
    ciphersuite::CURVE25519_KEY_LEN, context::KKTContext, error::KKTError, frame::KKTFrame,
};
use blake3::Hasher;
use libcrux_chacha20poly1305::{NONCE_LEN, TAG_LEN};
use nym_crypto::asymmetric::x25519;
use rand::{CryptoRng, RngCore};
use zeroize::Zeroize;

#[derive(Clone, Copy, Zeroize)]
pub struct KKTSessionSecret([u8; 32]);

impl KKTSessionSecret {
    pub fn new<R>(rng: &mut R, remote_public_key: &x25519::PublicKey) -> (Self, x25519::PublicKey)
    where
        R: RngCore + CryptoRng,
    {
        let mut private_key_bytes = [0u8; x25519::PRIVATE_KEY_SIZE];
        rng.fill_bytes(&mut private_key_bytes);

        let ephemeral_private_key = x25519::PrivateKey::from_secret(private_key_bytes);
        let ephemeral_public_key = x25519::PublicKey::from(&ephemeral_private_key);

        (
            Self::derive(&ephemeral_private_key, remote_public_key),
            ephemeral_public_key,
        )
    }
    pub fn from_bytes(secret: [u8; 32]) -> Self {
        Self(secret)
    }

    fn try_derive(private_key: &x25519::PrivateKey, public_key: &[u8]) -> Result<Self, KKTError> {
        let mut pub_key: [u8; 32] = [0u8; 32];
        pub_key.copy_from_slice(&public_key[0..CURVE25519_KEY_LEN]);

        // Todo: check validity of pk...
        let pk = x25519::PublicKey::from(pub_key);
        Ok(Self::derive(private_key, &pk))
    }

    pub fn derive(private_key: &x25519::PrivateKey, public_key: &x25519::PublicKey) -> Self {
        let mut shared_secret = private_key.diffie_hellman(public_key);

        let mut hasher = Hasher::new();

        hasher.update(&shared_secret);
        shared_secret.zeroize();

        Self(hasher.finalize().as_bytes().to_owned())
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

pub fn encrypt_initial_kkt_frame<R>(
    rng: &mut R,
    remote_public_key: &x25519::PublicKey,
    kkt_frame: &KKTFrame,
) -> Result<(KKTSessionSecret, Vec<u8>), KKTError>
where
    R: CryptoRng + RngCore,
{
    let (session_secret_key, ephemeral_public_key) = KKTSessionSecret::new(rng, remote_public_key);

    let mut encrypted_frame =
        encrypt_kkt_frame(rng, &session_secret_key, kkt_frame, KKT_INITIAL_FRAME_AAD)?;

    let mut output_buffer = Vec::with_capacity(encrypted_frame.len() + CURVE25519_KEY_LEN);
    output_buffer.extend_from_slice(ephemeral_public_key.as_bytes());
    output_buffer.append(&mut encrypted_frame);

    // [     32     |  12   | ciphertext | 16];
    // [eph_pub_key | nonce | ciphertext | tag];
    Ok((session_secret_key, output_buffer))
}

pub fn decrypt_initial_kkt_frame(
    responder_private_key: &x25519::PrivateKey,
    encrypted_frame_bytes: &[u8],
) -> Result<(KKTSessionSecret, KKTFrame, KKTContext), KKTError> {
    if encrypted_frame_bytes.len() < CURVE25519_KEY_LEN + TAG_LEN + NONCE_LEN {
        Err(KKTError::AEADError {
            info: "Encrypted KKT Frame is too short.",
        })
    } else {
        let shared_secret = KKTSessionSecret::try_derive(
            responder_private_key,
            &encrypted_frame_bytes[0..CURVE25519_KEY_LEN],
        )?;

        let (kkt_frame, kkt_context) = decrypt_kkt_frame(
            &shared_secret,
            &encrypted_frame_bytes[CURVE25519_KEY_LEN..],
            KKT_INITIAL_FRAME_AAD,
        )?;
        Ok((shared_secret, kkt_frame, kkt_context))
    }
}

pub fn encrypt_kkt_frame<R>(
    rng: &mut R,
    secret_key: &KKTSessionSecret,
    kkt_frame: &KKTFrame,
    aad: &[u8],
) -> Result<Vec<u8>, KKTError>
where
    R: CryptoRng + RngCore,
{
    let kkt_frame_bytes = kkt_frame.to_bytes();

    // generate nonce
    let mut nonce: [u8; NONCE_LEN] = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce);

    let mut ciphertext = encrypt(secret_key.as_bytes(), &kkt_frame_bytes, aad, &nonce)?;

    // [  12  | ciphertext | 16];
    // [nonce | ciphertext | tag];
    let mut output_buffer: Vec<u8> =
        Vec::with_capacity(NONCE_LEN + kkt_frame_bytes.len() + TAG_LEN);

    output_buffer.extend_from_slice(&nonce);
    output_buffer.append(&mut ciphertext);

    Ok(output_buffer)
}

// kkt_frame_bytes should look like this
// [  12  | ciphertext | 16];
// [nonce | ciphertext | tag];
pub fn decrypt_kkt_frame(
    secret_key: &KKTSessionSecret,
    kkt_frame_bytes: &[u8],
    aad: &[u8],
) -> Result<(KKTFrame, KKTContext), KKTError> {
    let mut nonce: [u8; NONCE_LEN] = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&kkt_frame_bytes[0..NONCE_LEN]);

    let plaintext = decrypt(
        secret_key.as_bytes(),
        &kkt_frame_bytes[NONCE_LEN..],
        aad,
        &nonce,
    )?;

    KKTFrame::from_bytes(&plaintext)
}

fn encrypt(
    secret_key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
    nonce: &[u8; NONCE_LEN],
) -> Result<Vec<u8>, KKTError> {
    let mut output_buffer = vec![0; plaintext.len() + TAG_LEN];
    libcrux_chacha20poly1305::encrypt(secret_key, plaintext, &mut output_buffer, aad, nonce)?;
    Ok(output_buffer)
}

fn decrypt(
    secret_key: &[u8; 32],
    ciphertext: &[u8],
    aad: &[u8],
    nonce: &[u8; NONCE_LEN],
) -> Result<Vec<u8>, KKTError> {
    let mut output_buffer = vec![0; ciphertext.len() - TAG_LEN];
    libcrux_chacha20poly1305::decrypt(secret_key, &mut output_buffer, ciphertext, aad, nonce)?;
    Ok(output_buffer)
}

#[cfg(test)]
mod test {
    use crate::ciphersuite::Ciphersuite;
    use crate::context::{KKTContext, KKTMode, KKTRole};
    use crate::encryption::{decrypt_kkt_frame, encrypt_kkt_frame};
    use crate::frame::{KKT_SESSION_ID_LEN, KKTFrame};
    use crate::{
        ciphersuite::HASH_LEN_256,
        encryption::{KKTSessionSecret, decrypt, encrypt},
        key_utils::generate_keypair_x25519,
    };
    use rand::{RngCore, SeedableRng, rng};
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_keygen() {
        let mut rng = rng();
        let responder_x25519_keypair = generate_keypair_x25519(&mut rng);

        let (session_secret_key, ephemeral_public_key) =
            KKTSessionSecret::new(&mut rng, &responder_x25519_keypair.public_key());

        let shared_secret = KKTSessionSecret::try_derive(
            responder_x25519_keypair.private_key(),
            ephemeral_public_key.as_bytes().as_slice(),
        )
        .unwrap();

        assert_eq!(shared_secret.as_bytes(), session_secret_key.as_bytes())
    }

    #[test]
    fn test_encryption() {
        let mut rng = rng();

        let mut secret_key = [0u8; HASH_LEN_256];
        rng.fill_bytes(&mut secret_key);

        let mut plaintext = vec![0; 100];
        rng.fill_bytes(&mut plaintext);

        let mut nonce = [0; 12];
        rng.fill_bytes(&mut nonce);

        let mut aad = vec![0; 124];
        rng.fill_bytes(&mut aad);

        let ciphertext = encrypt(&secret_key, &plaintext, &aad, &nonce).unwrap();

        let o_plaintext = decrypt(&secret_key, &ciphertext, &aad, &nonce).unwrap();

        assert_eq!(o_plaintext, plaintext)
    }

    #[test]
    fn kkt_frame_encryption() -> anyhow::Result<()> {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let session_key = KKTSessionSecret::from_bytes([42u8; 32]);
        let aad = b"my-amazing-aad";

        let valid_context = KKTContext::new(
            KKTRole::Initiator,
            KKTMode::Mutual,
            Ciphersuite::decode([255, 1, 0, 0])?,
        )?;
        let dummy_frame = KKTFrame::new(
            valid_context.encode()?,
            &[2u8; 32],
            [3u8; KKT_SESSION_ID_LEN],
            &[4u8; 64],
        );

        let ciphertext = encrypt_kkt_frame(&mut rng, &session_key, &dummy_frame, aad.as_slice())?;

        let (frame, context) = decrypt_kkt_frame(&session_key, &ciphertext, aad.as_slice())?;

        assert_eq!(dummy_frame, frame);
        assert_eq!(context, valid_context);
        Ok(())
    }
}
