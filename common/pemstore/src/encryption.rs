// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Encryption of private key files at rest, under either a passphrase or a raw 256-bit key.
//!
//! Both kinds of encrypted files share the `ENCRYPTED <TYPE>` PEM tag and are told apart by the
//! first byte of their contents:
//!
//! - passphrase: DER `EncryptedPrivateKeyInfo` (PBES2: scrypt + AES-256-GCM), which always
//!   starts with the DER `SEQUENCE` tag, `0x30`.
//! - raw key: `0x01 || nonce (12 bytes) || AES-256-GCM ciphertext and tag`, with the version byte
//!   and the PEM tag as associated data. The key is used as is, without key derivation, so it
//!   must be uniformly random, e.g. generated and kept by the platform keychain.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use pkcs8::EncryptedPrivateKeyInfoOwned;
use pkcs8::der::asn1::OctetString;
use pkcs8::der::{Decode, Encode};
use pkcs8::pkcs5::{pbes2, scrypt};
use std::fmt;
use std::io;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Length of a [`RawKey`] in bytes.
pub const RAW_KEY_LEN: usize = 32;

/// 128-bit scrypt salt, fresh for every encrypted file.
const SCRYPT_SALT_LEN: usize = 16;

/// 96-bit AES-GCM nonce, for both formats.
const GCM_NONCE_LEN: usize = 12;

/// 128-bit AES-GCM authentication tag.
const GCM_TAG_LEN: usize = 16;

/// First byte of a DER `SEQUENCE`, i.e. of a passphrase-encrypted `EncryptedPrivateKeyInfo`.
const DER_SEQUENCE_TAG: u8 = 0x30;

/// First byte of a raw-key-encrypted file, versioning its layout.
const RAW_KEY_FORMAT_V1: u8 = 0x01;

/// Passphrase protecting private key files on disk.
#[derive(Clone)]
pub struct Passphrase(Zeroizing<String>);

impl Passphrase {
    pub fn new(passphrase: impl Into<String>) -> Self {
        Passphrase(Zeroizing::new(passphrase.into()))
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl From<String> for Passphrase {
    fn from(passphrase: String) -> Self {
        Passphrase::new(passphrase)
    }
}

impl fmt::Debug for Passphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Passphrase(..)")
    }
}

/// Raw 256-bit key protecting private key files on disk. It is used as is, without passphrase
/// key derivation, so it must be uniformly random.
///
/// The key is kept in a single shared heap allocation, so moving a `RawKey` doesn't leave copies
/// of it behind, and clones share it. It's zeroized when the last clone is dropped.
#[derive(Clone)]
pub struct RawKey(Arc<Zeroizing<[u8; RAW_KEY_LEN]>>);

impl RawKey {
    /// Copies the key from a borrowed buffer straight into the shared allocation, without
    /// intermediate copies. The caller stays responsible for wiping its own buffer.
    pub fn from_bytes(key: &[u8; RAW_KEY_LEN]) -> Self {
        RawKey::from_fn(|buf| buf.copy_from_slice(key))
    }

    /// Fills the key in place, e.g. from an RNG, so it only ever exists in the shared allocation.
    pub fn from_fn(fill: impl FnOnce(&mut [u8; RAW_KEY_LEN])) -> Self {
        let mut key = Arc::new(Zeroizing::new([0u8; RAW_KEY_LEN]));
        // the Arc isn't shared yet, so this doesn't clone
        fill(&mut *Arc::make_mut(&mut key));
        RawKey(key)
    }

    pub(crate) fn as_bytes(&self) -> &[u8; RAW_KEY_LEN] {
        &self.0
    }
}

/// Copies the key straight into the shared allocation. Fails unless it's exactly [`RAW_KEY_LEN`]
/// bytes. The caller stays responsible for wiping its own buffer.
impl TryFrom<&[u8]> for RawKey {
    type Error = InvalidRawKeyLength;

    fn try_from(key: &[u8]) -> Result<Self, Self::Error> {
        if key.len() != RAW_KEY_LEN {
            return Err(InvalidRawKeyLength { len: key.len() });
        }
        Ok(RawKey::from_fn(|buf| buf.copy_from_slice(key)))
    }
}

/// Returned by `RawKey::try_from(&[u8])` when the input isn't exactly [`RAW_KEY_LEN`] bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the raw key must be exactly {RAW_KEY_LEN} bytes, got {len}")]
pub struct InvalidRawKeyLength {
    pub len: usize,
}

impl fmt::Debug for RawKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawKey(..)")
    }
}

/// Secret protecting private key files on disk.
#[derive(Clone, Debug)]
pub enum KeySecret {
    /// Human-chosen passphrase, stretched with scrypt.
    Passphrase(Passphrase),

    /// Uniformly random 256-bit key, used without key derivation.
    RawKey(RawKey),
}

impl From<Passphrase> for KeySecret {
    fn from(passphrase: Passphrase) -> Self {
        KeySecret::Passphrase(passphrase)
    }
}

impl From<RawKey> for KeySecret {
    fn from(key: RawKey) -> Self {
        KeySecret::RawKey(key)
    }
}

/// Encrypts raw key bytes into the contents of an encrypted PEM file labelled `pem_tag`.
pub(crate) fn encrypt(secret: &KeySecret, plaintext: &[u8], pem_tag: &str) -> io::Result<Vec<u8>> {
    match secret {
        KeySecret::Passphrase(passphrase) => encrypt_with_passphrase(passphrase, plaintext),
        KeySecret::RawKey(key) => encrypt_with_raw_key(key, plaintext, pem_tag),
    }
}

/// Decrypts the contents of an encrypted PEM file labelled `pem_tag`, produced by [`encrypt`],
/// back into raw key bytes.
pub(crate) fn decrypt(
    secret: &KeySecret,
    contents: &[u8],
    pem_tag: &str,
) -> io::Result<Zeroizing<Vec<u8>>> {
    match (secret, contents.first()) {
        (KeySecret::Passphrase(passphrase), Some(&DER_SEQUENCE_TAG)) => {
            decrypt_with_passphrase(passphrase, contents)
        }
        (KeySecret::RawKey(key), Some(&RAW_KEY_FORMAT_V1)) => {
            decrypt_with_raw_key(key, contents, pem_tag)
        }
        (KeySecret::Passphrase(_), Some(&RAW_KEY_FORMAT_V1)) => Err(invalid_data(
            "the key is encrypted with a raw key, not a passphrase",
        )),
        (KeySecret::RawKey(_), Some(&DER_SEQUENCE_TAG)) => Err(invalid_data(
            "the key is encrypted with a passphrase, not a raw key",
        )),
        _ => Err(invalid_data("unrecognised encrypted key format")),
    }
}

/// Encrypts into a DER `EncryptedPrivateKeyInfo` (PBES2: scrypt + AES-256-GCM).
fn encrypt_with_passphrase(passphrase: &Passphrase, plaintext: &[u8]) -> io::Result<Vec<u8>> {
    let salt: [u8; SCRYPT_SALT_LEN] = rand::random();
    let nonce: [u8; GCM_NONCE_LEN] = rand::random();

    // OWASP-recommended cost (N = 2^17, r = 8, p = 1, about 128 MiB per derivation)
    let params = pbes2::Parameters::scrypt_aes256gcm(scrypt::Params::RECOMMENDED, &salt, nonce)
        .map_err(io::Error::other)?;
    let ciphertext = params
        .encrypt(passphrase.as_bytes(), plaintext)
        .map_err(io::Error::other)?;

    let info = EncryptedPrivateKeyInfoOwned {
        encryption_algorithm: params.into(),
        encrypted_data: OctetString::new(ciphertext).map_err(io::Error::other)?,
    };
    info.to_der().map_err(io::Error::other)
}

fn decrypt_with_passphrase(passphrase: &Passphrase, der: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    let info = EncryptedPrivateKeyInfoOwned::from_der(der)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    // not `info.decrypt()`: that insists the plaintext is itself DER, and ours is raw bytes
    info.encryption_algorithm
        .decrypt(passphrase.as_bytes(), info.encrypted_data.as_bytes())
        .map(Zeroizing::new)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

/// Encrypts into `version || nonce || ciphertext and tag`.
fn encrypt_with_raw_key(key: &RawKey, plaintext: &[u8], pem_tag: &str) -> io::Result<Vec<u8>> {
    let nonce: [u8; GCM_NONCE_LEN] = rand::random();

    let ciphertext = raw_key_cipher(key)
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: plaintext,
                aad: &raw_key_aad(pem_tag),
            },
        )
        .map_err(|_| io::Error::other("failed to encrypt the key"))?;

    let mut contents = Vec::with_capacity(1 + GCM_NONCE_LEN + ciphertext.len());
    contents.push(RAW_KEY_FORMAT_V1);
    contents.extend_from_slice(&nonce);
    contents.extend_from_slice(&ciphertext);
    Ok(contents)
}

fn decrypt_with_raw_key(
    key: &RawKey,
    contents: &[u8],
    pem_tag: &str,
) -> io::Result<Zeroizing<Vec<u8>>> {
    if contents.len() < 1 + GCM_NONCE_LEN + GCM_TAG_LEN {
        return Err(invalid_data("the encrypted key is truncated"));
    }
    let (nonce, ciphertext) = contents[1..].split_at(GCM_NONCE_LEN);
    let nonce = Nonce::try_from(nonce).map_err(|_| invalid_data("invalid nonce"))?;

    raw_key_cipher(key)
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: &raw_key_aad(pem_tag),
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| invalid_data("authentication failed"))
}

fn raw_key_cipher(key: &RawKey) -> Aes256Gcm {
    Aes256Gcm::new(key.as_bytes().into())
}

/// Binds the ciphertext to its format version and PEM tag, so it can't be relabelled as
/// another kind of key.
fn raw_key_aad(pem_tag: &str) -> Vec<u8> {
    let mut aad = Vec::with_capacity(1 + pem_tag.len());
    aad.push(RAW_KEY_FORMAT_V1);
    aad.extend_from_slice(pem_tag.as_bytes());
    aad
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG: &str = "ENCRYPTED DUMMY KEY";

    fn passphrase(passphrase: &str) -> KeySecret {
        Passphrase::new(passphrase).into()
    }

    fn raw_key(byte: u8) -> KeySecret {
        RawKey::from_bytes(&[byte; RAW_KEY_LEN]).into()
    }

    #[test]
    fn raw_key_from_slice_checks_the_length() {
        let key = RawKey::try_from(&[7; RAW_KEY_LEN][..]).unwrap();

        assert_eq!(key.as_bytes(), &[7; RAW_KEY_LEN]);
        assert_eq!(
            RawKey::try_from(&[7; RAW_KEY_LEN - 1][..]).unwrap_err(),
            InvalidRawKeyLength {
                len: RAW_KEY_LEN - 1
            }
        );
        assert!(RawKey::try_from(&[7; RAW_KEY_LEN + 1][..]).is_err());
        assert!(RawKey::try_from(&[][..]).is_err());
    }

    #[test]
    fn raw_key_constructors_agree() {
        let filled = RawKey::from_fn(|key| key.fill(7));

        assert_eq!(filled.as_bytes(), &[7; RAW_KEY_LEN]);
        assert_eq!(
            RawKey::from_bytes(&[7; RAW_KEY_LEN]).as_bytes(),
            filled.as_bytes()
        );
    }

    #[test]
    fn round_trips_under_the_same_passphrase() {
        let secret = passphrase("hunter2");

        let encrypted = encrypt(&secret, b"raw key bytes", TAG).unwrap();

        assert_eq!(encrypted[0], DER_SEQUENCE_TAG);
        assert_eq!(
            decrypt(&secret, &encrypted, TAG).unwrap().as_slice(),
            b"raw key bytes"
        );
    }

    #[test]
    fn fails_under_a_different_passphrase() {
        let encrypted = encrypt(&passphrase("hunter2"), b"raw key bytes", TAG).unwrap();

        assert!(decrypt(&passphrase("hunter3"), &encrypted, TAG).is_err());
    }

    #[test]
    fn round_trips_under_the_same_raw_key() {
        let secret = raw_key(1);

        let encrypted = encrypt(&secret, b"raw key bytes", TAG).unwrap();

        assert_eq!(encrypted[0], RAW_KEY_FORMAT_V1);
        assert_eq!(
            encrypted.len(),
            1 + GCM_NONCE_LEN + b"raw key bytes".len() + GCM_TAG_LEN
        );
        assert_eq!(
            decrypt(&secret, &encrypted, TAG).unwrap().as_slice(),
            b"raw key bytes"
        );
    }

    #[test]
    fn raw_key_encryption_uses_a_fresh_nonce() {
        let secret = raw_key(1);

        let first = encrypt(&secret, b"raw key bytes", TAG).unwrap();
        let second = encrypt(&secret, b"raw key bytes", TAG).unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn fails_under_a_different_raw_key() {
        let encrypted = encrypt(&raw_key(1), b"raw key bytes", TAG).unwrap();

        assert!(decrypt(&raw_key(2), &encrypted, TAG).is_err());
    }

    #[test]
    fn raw_key_ciphertext_is_bound_to_its_pem_tag() {
        let secret = raw_key(1);
        let encrypted = encrypt(&secret, b"raw key bytes", TAG).unwrap();

        assert!(decrypt(&secret, &encrypted, "ENCRYPTED OTHER KEY").is_err());
    }

    #[test]
    fn raw_key_rejects_tampered_or_truncated_contents() {
        let secret = raw_key(1);
        let encrypted = encrypt(&secret, b"raw key bytes", TAG).unwrap();

        let mut tampered = encrypted.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert!(decrypt(&secret, &tampered, TAG).is_err());

        assert!(decrypt(&secret, &encrypted[..1 + GCM_NONCE_LEN], TAG).is_err());
        assert!(decrypt(&secret, &[], TAG).is_err());
    }

    #[test]
    fn rejects_the_other_kind_of_secret() {
        let by_passphrase = encrypt(&passphrase("hunter2"), b"raw key bytes", TAG).unwrap();
        let by_raw_key = encrypt(&raw_key(1), b"raw key bytes", TAG).unwrap();

        let err = decrypt(&raw_key(1), &by_passphrase, TAG).unwrap_err();
        assert!(err.to_string().contains("not a raw key"), "{err}");

        let err = decrypt(&passphrase("hunter2"), &by_raw_key, TAG).unwrap_err();
        assert!(err.to_string().contains("not a passphrase"), "{err}");
    }
}
