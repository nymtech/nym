// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use aes_gcm::aead::generic_array::ArrayLength;
use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{
  password_hash::rand_core::{OsRng, RngCore},
  Algorithm, Argon2, Params, Version,
};
use serde::{Deserialize, Serialize};

const MEMORY_COST: u32 = 16 * 1024;
const ITERATIONS: u32 = 3;
const PARALLELISM: u32 = 1;
const OUTPUT_LENGTH: usize = 32;

// as per Argon2 recommendation
const SALT_LEN: usize = 16;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EncryptedData {
  #[serde(with = "base64")]
  ciphertext: Vec<u8>,
  #[serde(with = "base64")]
  salt: Vec<u8>,
  #[serde(with = "base64")]
  iv: Vec<u8>,
}

mod base64 {
  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::encode(bytes))
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let s = <&str>::deserialize(deserializer)?;
    base64::decode(s).map_err(serde::de::Error::custom)
  }
}

fn derive_cipher_key<KeySize>(password: &str, salt: &[u8]) -> Result<Key<KeySize>, BackendError>
where
  KeySize: ArrayLength<u8>,
{
  // this can only fail if output length is either smaller than 4 or larger than 2^32 - 1 which is not the case here
  let params = Params::new(MEMORY_COST, ITERATIONS, PARALLELISM, Some(OUTPUT_LENGTH)).unwrap();

  let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

  let mut key = Key::default();
  argon2.hash_password_into(password.as_bytes(), salt, &mut key)?;

  Ok(key)
}

pub(crate) fn encrypt(data: &[u8], password: &str) -> Result<EncryptedData, BackendError> {
  let mut rng = OsRng;

  let mut salt = [0u8; SALT_LEN];
  rng.fill_bytes(&mut salt);

  let key = derive_cipher_key(password, &salt)?;

  let mut iv = Nonce::default();
  rng.fill_bytes(&mut iv);

  let cipher = Aes256Gcm::new(&key);

  let ciphertext = cipher
    .encrypt(&iv, data)
    .map_err(|_| BackendError::EncryptionError)?;

  Ok(EncryptedData {
    ciphertext,
    salt: salt.to_vec(),
    iv: iv.to_vec(),
  })
}

pub(crate) fn decrypt(
  encrypted_data: &EncryptedData,
  password: &str,
) -> Result<Vec<u8>, BackendError> {
  let key = derive_cipher_key(password, &encrypted_data.salt)?;
  let cipher = Aes256Gcm::new(&key);

  cipher
    .decrypt(
      Nonce::from_slice(&encrypted_data.iv),
      encrypted_data.ciphertext.as_ref(),
    )
    .map_err(|_| BackendError::DecryptionError)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn data_encryption() {
    let password = "my-super-secret-password";
    let payload = b"my secret message";

    let wrong_password = "brute-force-attempt-1";

    let mut encrypted_data = encrypt(payload, password).unwrap();
    let recovered = decrypt(&encrypted_data, password).unwrap();
    assert_eq!(payload.to_vec(), recovered);

    // decryption with wrong password fails
    assert!(decrypt(&encrypted_data, wrong_password).is_err());

    // decryption fails if ciphertext got malformed
    encrypted_data.ciphertext[3] ^= 123;
    assert!(decrypt(&encrypted_data, wrong_password).is_err());

    // restore the ciphertext (for test purposes)
    encrypted_data.ciphertext[3] ^= 123;

    // decryption fails if salt got malformed (it would result in incorrect key being derived)
    encrypted_data.salt[3] ^= 123;
    assert!(decrypt(&encrypted_data, password).is_err());

    // restore the salt (for test purposes)
    encrypted_data.salt[3] ^= 123;

    // decryption fails if iv got malformed
    encrypted_data.iv[3] ^= 123;
    assert!(decrypt(&encrypted_data, password).is_err());
  }
}
