// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use pkcs8::EncryptedPrivateKeyInfoOwned;
use pkcs8::der::asn1::OctetString;
use pkcs8::der::{Decode, Encode};
use pkcs8::pkcs5::{pbes2, scrypt};
use std::fmt;
use std::io;
use zeroize::Zeroizing;

/// 128-bit scrypt salt, fresh for every encrypted file.
const SCRYPT_SALT_LEN: usize = 16;

/// 96-bit AES-GCM nonce, the size `scrypt_aes256gcm` expects.
const GCM_NONCE_LEN: usize = 12;

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

/// Encrypts raw key bytes into a DER `EncryptedPrivateKeyInfo` (PBES2: scrypt + AES-256-GCM).
pub(crate) fn encrypt(passphrase: &Passphrase, plaintext: &[u8]) -> io::Result<Vec<u8>> {
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

/// Decrypts a DER `EncryptedPrivateKeyInfo` produced by [`encrypt`] back into raw key bytes.
pub(crate) fn decrypt(passphrase: &Passphrase, der: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    let info = EncryptedPrivateKeyInfoOwned::from_der(der)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    // not `info.decrypt()`: that insists the plaintext is itself DER, and ours is raw bytes
    info.encryption_algorithm
        .decrypt(passphrase.as_bytes(), info.encrypted_data.as_bytes())
        .map(Zeroizing::new)
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to decrypt the key (wrong passphrase?): {err}"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_under_the_same_passphrase() {
        let passphrase = Passphrase::new("hunter2");

        let encrypted = encrypt(&passphrase, b"raw key bytes").unwrap();

        assert_ne!(encrypted.as_slice(), b"raw key bytes");
        assert_eq!(
            decrypt(&passphrase, &encrypted).unwrap().as_slice(),
            b"raw key bytes"
        );
    }

    #[test]
    fn fails_under_a_different_passphrase() {
        let encrypted = encrypt(&Passphrase::new("hunter2"), b"raw key bytes").unwrap();

        assert!(decrypt(&Passphrase::new("hunter3"), &encrypted).is_err());
    }
}
