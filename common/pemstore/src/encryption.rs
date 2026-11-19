// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Encryption of private key files at rest, under either a passphrase or a raw 256-bit key.
//!
//! Both kinds of secret produce the same format: a DER `EncryptedPrivateKeyInfo` using PBES2 with
//! AES-256-GCM, under the `ENCRYPTED <TYPE>` PEM tag. They differ only in how the AES key is
//! derived, which the KDF recorded in the file tells apart:
//!
//! - passphrase: scrypt, at the OWASP-recommended cost.
//! - raw key: PBKDF2-HMAC-SHA256 with a single iteration. The raw key must already be uniformly
//!   random, e.g. generated and kept by the platform keychain, so it needs no stretching. The
//!   single HMAC over a fresh salt just gives every file its own AES key.

use pkcs8::EncryptedPrivateKeyInfoOwned;
use pkcs8::der::asn1::OctetString;
use pkcs8::der::{Decode, Encode};
use pkcs8::pkcs5::{EncryptionScheme, pbes2, scrypt};
use rand::CryptoRng;
use std::fmt;
use std::io;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Length of a [`RawKey`] in bytes.
pub const RAW_KEY_LEN: usize = 32;

/// 128-bit KDF salt, fresh for every encrypted file.
const KDF_SALT_LEN: usize = 16;

/// 96-bit AES-GCM nonce.
const GCM_NONCE_LEN: usize = 12;

/// PBKDF2 iterations for a raw key, which is already uniformly random and needs no stretching.
const RAW_KEY_PBKDF2_ITERATIONS: u32 = 1;

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

/// Raw 256-bit key protecting private key files on disk. It isn't stretched like a passphrase,
/// so it must be uniformly random.
///
/// The key is kept in a single shared heap allocation, so moving a `RawKey` doesn't leave copies
/// of it behind, and clones share it. It's zeroized when the last clone is dropped.
#[derive(Clone)]
pub struct RawKey(Arc<Zeroizing<[u8; RAW_KEY_LEN]>>);

impl RawKey {
    /// Copies the key from a borrowed buffer straight into the shared allocation, without
    /// intermediate copies. The caller stays responsible for wiping its own buffer.
    pub fn from_bytes(key: &[u8; RAW_KEY_LEN]) -> Self {
        RawKey(with_heap_key_buf(|buf| buf.copy_from_slice(key)))
    }

    /// Generates a new uniformly random key, filled straight into the shared allocation so it never
    /// exists anywhere else.
    pub fn generate<R: CryptoRng + ?Sized>(rng: &mut R) -> Self {
        RawKey(with_heap_key_buf(|buf| rng.fill_bytes(buf)))
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
        if key.len() == RAW_KEY_LEN {
            Ok(RawKey(with_heap_key_buf(|buf| buf.copy_from_slice(key))))
        } else {
            Err(InvalidRawKeyLength { len: key.len() })
        }
    }
}

/// Allocates zeroed storage on heap and passes it to `initialize` for initialization which must fill in the buffer.
/// Returns initialized key buffer stored on heap
fn with_heap_key_buf(
    initialize: impl FnOnce(&mut [u8; RAW_KEY_LEN]),
) -> Arc<Zeroizing<[u8; RAW_KEY_LEN]>> {
    let mut storage = Arc::new(Zeroizing::new([0u8; RAW_KEY_LEN]));

    // It's safe because only one instance of Arc exists
    #[allow(clippy::expect_used)]
    let buf = Arc::get_mut(&mut storage).expect("failed to Arc::get_mut()");

    initialize(buf);
    storage
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

    /// Uniformly random 256-bit key, used without key stretching.
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

/// Encrypts raw key bytes into a DER `EncryptedPrivateKeyInfo` (PBES2 + AES-256-GCM).
pub(crate) fn encrypt(secret: &KeySecret, plaintext: &[u8]) -> io::Result<Vec<u8>> {
    let salt: [u8; KDF_SALT_LEN] = rand::random();
    let nonce: [u8; GCM_NONCE_LEN] = rand::random();

    let (kdf, password): (pbes2::Kdf, &[u8]) = match secret {
        // OWASP-recommended cost (N = 2^17, r = 8, p = 1, about 128 MiB per derivation)
        KeySecret::Passphrase(passphrase) => (
            pbes2::ScryptParams::from_params_and_salt(scrypt::Params::RECOMMENDED, &salt)
                .map_err(io::Error::other)?
                .into(),
            passphrase.as_bytes(),
        ),
        // already uniformly random: a single HMAC is enough to give each file its own key
        KeySecret::RawKey(key) => (
            pbes2::Pbkdf2Params::hmac_sha256(RAW_KEY_PBKDF2_ITERATIONS, &salt)
                .map_err(io::Error::other)?
                .into(),
            key.as_bytes(),
        ),
    };
    let params = pbes2::Parameters {
        kdf,
        encryption: pbes2::EncryptionScheme::Aes256Gcm { nonce },
    };
    let ciphertext = params
        .encrypt(password, plaintext)
        .map_err(io::Error::other)?;

    let info = EncryptedPrivateKeyInfoOwned {
        encryption_algorithm: params.into(),
        encrypted_data: OctetString::new(ciphertext).map_err(io::Error::other)?,
    };
    info.to_der().map_err(io::Error::other)
}

/// Decrypts a DER `EncryptedPrivateKeyInfo` produced by [`encrypt`] back into raw key bytes.
pub(crate) fn decrypt(secret: &KeySecret, der: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    let info = EncryptedPrivateKeyInfoOwned::from_der(der)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let EncryptionScheme::Pbes2(params) = &info.encryption_algorithm else {
        return Err(invalid_data("unrecognised encrypted key format"));
    };
    if !matches!(params.encryption, pbes2::EncryptionScheme::Aes256Gcm { .. }) {
        return Err(invalid_data("unrecognised encrypted key format"));
    }

    // checked before deriving anything, so a raw key never runs the passphrase's scrypt
    let password: &[u8] = match (secret, &params.kdf) {
        (KeySecret::Passphrase(passphrase), pbes2::Kdf::Scrypt(_)) => passphrase.as_bytes(),
        (KeySecret::RawKey(key), pbes2::Kdf::Pbkdf2(kdf))
            if kdf.prf == pbes2::Pbkdf2Prf::HmacWithSha256
                && kdf.iteration_count == RAW_KEY_PBKDF2_ITERATIONS =>
        {
            key.as_bytes()
        }
        (KeySecret::Passphrase(_), pbes2::Kdf::Pbkdf2(_)) => {
            return Err(invalid_data(
                "the key is encrypted with a raw key, not a passphrase",
            ));
        }
        (KeySecret::RawKey(_), pbes2::Kdf::Scrypt(_)) => {
            return Err(invalid_data(
                "the key is encrypted with a passphrase, not a raw key",
            ));
        }
        _ => return Err(invalid_data("unrecognised encrypted key format")),
    };

    // not `info.decrypt()`: that insists the plaintext is itself DER, and ours is raw bytes
    params
        .decrypt(password, info.encrypted_data.as_bytes())
        .map(Zeroizing::new)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let expected = [7; RAW_KEY_LEN];

        assert_eq!(RawKey::from_bytes(&expected).as_bytes(), &expected);
        assert_eq!(
            RawKey::try_from(&expected[..]).unwrap().as_bytes(),
            &expected
        );
    }

    fn kdf_of(encrypted: &[u8]) -> pbes2::Kdf {
        let info = EncryptedPrivateKeyInfoOwned::from_der(encrypted).unwrap();
        let EncryptionScheme::Pbes2(params) = info.encryption_algorithm else {
            panic!("not PBES2");
        };
        assert!(matches!(
            params.encryption,
            pbes2::EncryptionScheme::Aes256Gcm { .. }
        ));
        params.kdf
    }

    #[test]
    fn generated_raw_keys_are_random() {
        let first = RawKey::generate(&mut rand::rng());
        let second = RawKey::generate(&mut rand::rng());

        assert_ne!(first.as_bytes(), second.as_bytes());
        assert_ne!(first.as_bytes(), &[0; RAW_KEY_LEN]);
    }

    #[test]
    fn round_trips_under_the_same_passphrase() {
        let secret = passphrase("hunter2");

        let encrypted = encrypt(&secret, b"raw key bytes").unwrap();

        assert!(kdf_of(&encrypted).is_scrypt());
        assert_eq!(
            decrypt(&secret, &encrypted).unwrap().as_slice(),
            b"raw key bytes"
        );
    }

    #[test]
    fn fails_under_a_different_passphrase() {
        let encrypted = encrypt(&passphrase("hunter2"), b"raw key bytes").unwrap();

        assert!(decrypt(&passphrase("hunter3"), &encrypted).is_err());
    }

    #[test]
    fn round_trips_under_the_same_raw_key() {
        let secret = raw_key(1);

        let encrypted = encrypt(&secret, b"raw key bytes").unwrap();

        let kdf = kdf_of(&encrypted);
        let pbkdf2 = kdf.pbkdf2().expect("raw keys use PBKDF2");
        assert_eq!(pbkdf2.prf, pbes2::Pbkdf2Prf::HmacWithSha256);
        assert_eq!(pbkdf2.iteration_count, RAW_KEY_PBKDF2_ITERATIONS);
        assert_eq!(
            decrypt(&secret, &encrypted).unwrap().as_slice(),
            b"raw key bytes"
        );
    }

    #[test]
    fn raw_key_encryption_uses_a_fresh_salt_and_nonce() {
        let secret = raw_key(1);

        let first = encrypt(&secret, b"raw key bytes").unwrap();
        let second = encrypt(&secret, b"raw key bytes").unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn fails_under_a_different_raw_key() {
        let encrypted = encrypt(&raw_key(1), b"raw key bytes").unwrap();

        assert!(decrypt(&raw_key(2), &encrypted).is_err());
    }

    #[test]
    fn rejects_tampered_or_truncated_contents() {
        let secret = raw_key(1);
        let encrypted = encrypt(&secret, b"raw key bytes").unwrap();

        let mut tampered = encrypted.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert!(decrypt(&secret, &tampered).is_err());

        assert!(decrypt(&secret, &encrypted[..encrypted.len() - 1]).is_err());
        assert!(decrypt(&secret, &[]).is_err());
    }

    #[test]
    fn raw_key_rejects_unexpected_pbkdf2_parameters() {
        let key = RawKey::from_bytes(&[1; RAW_KEY_LEN]);
        let params = pbes2::Parameters {
            kdf: pbes2::Pbkdf2Params::hmac_sha256(2, &[0; KDF_SALT_LEN])
                .unwrap()
                .into(),
            encryption: pbes2::EncryptionScheme::Aes256Gcm {
                nonce: [0; GCM_NONCE_LEN],
            },
        };
        let ciphertext = params.encrypt(key.as_bytes(), b"raw key bytes").unwrap();
        let encrypted = EncryptedPrivateKeyInfoOwned {
            encryption_algorithm: params.into(),
            encrypted_data: OctetString::new(ciphertext).unwrap(),
        }
        .to_der()
        .unwrap();

        let err = decrypt(&key.into(), &encrypted).unwrap_err();
        assert!(err.to_string().contains("unrecognised"), "{err}");
    }

    #[test]
    fn rejects_the_other_kind_of_secret() {
        let by_passphrase = encrypt(&passphrase("hunter2"), b"raw key bytes").unwrap();
        let by_raw_key = encrypt(&raw_key(1), b"raw key bytes").unwrap();

        let err = decrypt(&raw_key(1), &by_passphrase).unwrap_err();
        assert!(err.to_string().contains("not a raw key"), "{err}");

        let err = decrypt(&passphrase("hunter2"), &by_raw_key).unwrap_err();
        assert!(err.to_string().contains("not a passphrase"), "{err}");
    }
}
