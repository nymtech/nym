// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Pre-shared-key message layer.
//!
//! key.secret bytes -> HKDF-SHA-512 -> 256-bit XChaCha20-Poly1305 key.
//! Wire format (v2): `0x02 || nonce(24, random) || ciphertext || tag(16)`, version byte is AAD.
//! The encrypted plaintext is `len(4, big endian) || message || zero padding` rounded up to a
//! multiple of 1024 bytes, so the ciphertext length only reveals the message length in KiB steps.
//! Purely symmetric: nothing here is breakable by Shor's algorithm; Grover leaves >= 2^128 work.

use anyhow::{Context, Result, anyhow, bail};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use sha2::Sha512;
use std::path::Path;
use zeroize::Zeroizing;

pub const KEY_FILE: &str = "key.secret";

const MIN_SECRET_BYTES: usize = 64;
const GENERATED_SECRET_BYTES: usize = 64;
const VERSION: u8 = 2;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
const FINGERPRINT_LEN: usize = 4;
const LEN_PREFIX: usize = 4;
/// Padded plaintext granularity; every ciphertext is `OVERHEAD + k * PAD_BLOCK` bytes.
pub const PAD_BLOCK: usize = 1024;
/// Ciphertext bytes on top of the padded plaintext.
pub const OVERHEAD: usize = 1 + NONCE_LEN + TAG_LEN;
/// Longest message that still fits a single padding block.
pub const MAX_BLOCK_MESSAGE: usize = PAD_BLOCK - LEN_PREFIX;

const HKDF_SALT: &[u8] = b"nym-pq-chat/psk/v1";
const MESSAGE_KEY_INFO: &[u8] = b"nym-pq-chat xchacha20poly1305 message key";
const FINGERPRINT_INFO: &[u8] = b"nym-pq-chat key fingerprint";

pub struct PskCipher {
    aead: XChaCha20Poly1305,
    fingerprint: String,
}

impl PskCipher {
    pub fn from_key_file(path: &Path) -> Result<Self> {
        let raw = Zeroizing::new(
            std::fs::read(path)
                .with_context(|| format!("failed to read pre-shared key {}", path.display()))?,
        );
        Self::from_secret(raw.trim_ascii())
            .with_context(|| format!("invalid pre-shared key {}", path.display()))
    }

    pub fn from_secret(secret: &[u8]) -> Result<Self> {
        if secret.len() < MIN_SECRET_BYTES {
            bail!(
                "pre-shared key must be at least {MIN_SECRET_BYTES} bytes (got {}); create it with `nym-pq-chat keygen`",
                secret.len()
            );
        }
        let hkdf = Hkdf::<Sha512>::new(Some(HKDF_SALT), secret);

        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        hkdf.expand(MESSAGE_KEY_INFO, &mut key[..])
            .map_err(|err| anyhow!("hkdf expand failed: {err}"))?;

        let mut fingerprint = [0u8; FINGERPRINT_LEN];
        hkdf.expand(FINGERPRINT_INFO, &mut fingerprint)
            .map_err(|err| anyhow!("hkdf expand failed: {err}"))?;

        let aead = XChaCha20Poly1305::new_from_slice(&key[..])
            .map_err(|err| anyhow!("invalid derived key length: {err}"))?;

        Ok(Self {
            aead,
            fingerprint: hex::encode(fingerprint),
        })
    }

    /// Short public identifier of the key; equal on both machines iff key.secret is identical.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::fill(&mut nonce_bytes[..]);
        let nonce = XNonce::from(nonce_bytes);
        let padded = Zeroizing::new(pad(plaintext)?);

        let ciphertext = self
            .aead
            .encrypt(
                &nonce,
                Payload {
                    msg: &padded,
                    aad: &[VERSION],
                },
            )
            .map_err(|err| anyhow!("encryption failed: {err}"))?;

        let mut out = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
        out.push(VERSION);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let Some((&version, rest)) = data.split_first() else {
            bail!("empty message");
        };
        if version != VERSION {
            bail!("unsupported message format version {version} (expected {VERSION})");
        }
        if rest.len() < NONCE_LEN + TAG_LEN {
            bail!("message too short ({} bytes)", data.len());
        }
        let (nonce, ciphertext) = rest.split_at(NONCE_LEN);
        let nonce = XNonce::try_from(nonce).map_err(|_| anyhow!("invalid nonce length"))?;

        let padded = Zeroizing::new(
            self.aead
                .decrypt(
                    &nonce,
                    Payload {
                        msg: ciphertext,
                        aad: &[VERSION],
                    },
                )
                .map_err(|_| anyhow!("authentication failed: not encrypted with our key.secret"))?,
        );
        unpad(&padded)
    }
}

/// Padded plaintext length for a message of `len` bytes (always a multiple of `PAD_BLOCK`).
pub fn padded_len(len: usize) -> usize {
    (len + LEN_PREFIX).div_ceil(PAD_BLOCK) * PAD_BLOCK
}

fn pad(plaintext: &[u8]) -> Result<Vec<u8>> {
    let len = u32::try_from(plaintext.len()).context("message too long")?;
    let mut padded = vec![0u8; padded_len(plaintext.len())];
    padded[..LEN_PREFIX].copy_from_slice(&len.to_be_bytes());
    padded[LEN_PREFIX..LEN_PREFIX + plaintext.len()].copy_from_slice(plaintext);
    Ok(padded)
}

fn unpad(padded: &[u8]) -> Result<Vec<u8>> {
    if padded.len() < LEN_PREFIX || !padded.len().is_multiple_of(PAD_BLOCK) {
        bail!("malformed padding ({} bytes)", padded.len());
    }
    let len = u32::from_be_bytes([padded[0], padded[1], padded[2], padded[3]]) as usize;
    padded
        .get(LEN_PREFIX..LEN_PREFIX + len)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| anyhow!("malformed padding (length {len} exceeds {})", padded.len()))
}

/// Writes a fresh random key (hex) to `path`, mode 0600, refusing to overwrite an existing file.
pub fn generate_key_file(path: &Path) -> Result<()> {
    let mut secret = Zeroizing::new([0u8; GENERATED_SECRET_BYTES]);
    rand::fill(&mut secret[..]);
    let encoded = Zeroizing::new(format!("{}\n", hex::encode(&secret[..])));
    write_secret_file(path, encoded.as_bytes())
}

/// Creates `path` with mode 0600 (unix); fails if the file already exists.
pub fn write_secret_file(path: &Path, contents: &[u8]) -> Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const SECRET_A: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const SECRET_B: &[u8] = b"fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    #[test]
    fn roundtrip() {
        let cipher = PskCipher::from_secret(SECRET_A).unwrap();
        // empty, short, one block minus/plus one byte, ~2 KiB (one mixnet packet), 4 KiB, 32 KiB
        let sizes = [
            0,
            2,
            10,
            PAD_BLOCK - LEN_PREFIX,
            PAD_BLOCK - LEN_PREFIX + 1,
            2000,
            4096,
            32 * 1024,
        ];
        for size in sizes {
            let msg: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let ct = cipher.encrypt(&msg).unwrap();
            assert_eq!(ct.len(), padded_len(size) + OVERHEAD, "size {size}");
            assert_eq!(ct.len() % PAD_BLOCK, OVERHEAD);
            assert_eq!(ct[0], VERSION);
            assert_eq!(cipher.decrypt(&ct).unwrap(), msg, "size {size}");
        }
    }

    #[test]
    fn ciphertext_length_only_reveals_kib_steps() {
        let cipher = PskCipher::from_secret(SECRET_A).unwrap();
        let hi = cipher.encrypt(b"hi").unwrap();
        let hello = cipher.encrypt(b"hello").unwrap();
        assert_eq!(hi.len(), hello.len());
        assert_eq!(hi.len(), PAD_BLOCK + OVERHEAD);
        let full = cipher.encrypt(&[7u8; PAD_BLOCK - LEN_PREFIX]).unwrap();
        assert_eq!(full.len(), PAD_BLOCK + OVERHEAD);
        let over = cipher.encrypt(&[7u8; PAD_BLOCK - LEN_PREFIX + 1]).unwrap();
        assert_eq!(over.len(), 2 * PAD_BLOCK + OVERHEAD);
        assert_eq!(
            cipher.decrypt(&over).unwrap().len(),
            PAD_BLOCK - LEN_PREFIX + 1
        );
    }

    #[test]
    fn corrupt_padding_rejected() {
        assert!(unpad(&[]).is_err());
        assert!(unpad(&[0u8; PAD_BLOCK - 1]).is_err());
        let mut padded = pad(b"abc").unwrap();
        assert_eq!(unpad(&padded).unwrap(), b"abc");
        padded[..LEN_PREFIX].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(unpad(&padded).is_err());
    }

    #[test]
    fn ciphertexts_are_randomised() {
        let cipher = PskCipher::from_secret(SECRET_A).unwrap();
        let ct1 = cipher.encrypt(b"same").unwrap();
        let ct2 = cipher.encrypt(b"same").unwrap();
        assert_ne!(ct1, ct2);
        assert_ne!(ct1[1..1 + NONCE_LEN], ct2[1..1 + NONCE_LEN]);
    }

    #[test]
    fn plaintext_is_not_visible_in_ciphertext() {
        let cipher = PskCipher::from_secret(SECRET_A).unwrap();
        let msg = b"this exact sentence must never appear on the wire";
        let ct = cipher.encrypt(msg).unwrap();
        assert!(!ct.windows(8).any(|w| msg.windows(8).any(|m| m == w)));
    }

    #[test]
    fn wrong_key_fails() {
        let a = PskCipher::from_secret(SECRET_A).unwrap();
        let b = PskCipher::from_secret(SECRET_B).unwrap();
        let ct = a.encrypt(b"secret").unwrap();
        assert!(b.decrypt(&ct).is_err());
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn same_secret_same_fingerprint_and_interoperable() {
        let a = PskCipher::from_secret(SECRET_A).unwrap();
        let b = PskCipher::from_secret(SECRET_A).unwrap();
        assert_eq!(a.fingerprint(), b.fingerprint());
        assert_eq!(a.fingerprint().len(), FINGERPRINT_LEN * 2);
        assert_eq!(b.decrypt(&a.encrypt(b"x").unwrap()).unwrap(), b"x");
    }

    #[test]
    fn tampering_is_detected() {
        let cipher = PskCipher::from_secret(SECRET_A).unwrap();
        let ct = cipher.encrypt(b"integrity").unwrap();
        for i in 0..ct.len() {
            let mut tampered = ct.clone();
            tampered[i] ^= 0x01;
            assert!(cipher.decrypt(&tampered).is_err(), "byte {i}");
        }
        assert!(cipher.decrypt(&ct[..ct.len() - 1]).is_err());
        let mut extended = ct.clone();
        extended.push(0);
        assert!(cipher.decrypt(&extended).is_err());
    }

    #[test]
    fn malformed_inputs_rejected() {
        let cipher = PskCipher::from_secret(SECRET_A).unwrap();
        assert!(cipher.decrypt(&[]).is_err());
        assert!(cipher.decrypt(&[VERSION]).is_err());
        assert!(cipher.decrypt(&[VERSION; NONCE_LEN + TAG_LEN]).is_err());
        assert!(
            cipher
                .decrypt(&[VERSION + 1; 1 + NONCE_LEN + TAG_LEN])
                .is_err()
        );
        assert!(cipher.decrypt(&[VERSION; 1 + NONCE_LEN + TAG_LEN]).is_err());
        assert!(
            cipher
                .decrypt(b"plain unencrypted text that is long enough")
                .is_err()
        );
    }

    #[test]
    fn short_secret_rejected() {
        assert!(PskCipher::from_secret(b"").is_err());
        assert!(PskCipher::from_secret(&[7u8; MIN_SECRET_BYTES - 1]).is_err());
        assert!(PskCipher::from_secret(&[7u8; MIN_SECRET_BYTES]).is_ok());
    }

    #[test]
    fn key_file_generation_and_loading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE);

        generate_key_file(&path).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.len(), GENERATED_SECRET_BYTES * 2 + 1);
        assert!(hex::decode(contents.trim()).is_ok());
        assert!(generate_key_file(&path).is_err(), "must not overwrite");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        let from_file = PskCipher::from_key_file(&path).unwrap();
        let from_bytes = PskCipher::from_secret(contents.trim().as_bytes()).unwrap();
        assert_eq!(from_file.fingerprint(), from_bytes.fingerprint());
        let ct = from_file.encrypt(b"via usb stick").unwrap();
        assert_eq!(from_bytes.decrypt(&ct).unwrap(), b"via usb stick");
    }

    #[test]
    fn surrounding_whitespace_in_key_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE);
        std::fs::write(
            &path,
            format!("  \n{}\r\n\n", std::str::from_utf8(SECRET_A).unwrap()),
        )
        .unwrap();
        let from_file = PskCipher::from_key_file(&path).unwrap();
        let exact = PskCipher::from_secret(SECRET_A).unwrap();
        assert_eq!(from_file.fingerprint(), exact.fingerprint());
    }

    #[test]
    fn missing_key_file_errors() {
        assert!(PskCipher::from_key_file(Path::new("/nonexistent/key.secret")).is_err());
    }
}
