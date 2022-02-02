// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::password::UserPassword;
use crate::error::BackendError;
use aes_gcm::aead::generic_array::ArrayLength;
use aes_gcm::aead::{Aead, NewAead, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{
  password_hash::rand_core::{OsRng, RngCore},
  Algorithm, Argon2, Params, Version,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::convert::TryFrom;
use std::marker::PhantomData;
use zeroize::Zeroize;

const MEMORY_COST: u32 = 16 * 1024;
const ITERATIONS: u32 = 3;
const PARALLELISM: u32 = 1;
const OUTPUT_LENGTH: usize = 32;

// as per Argon2 recommendation
const SALT_LEN: usize = 16;

// AES256GCM Nonce is 96 bit long.
const IV_LEN: usize = 12;

#[derive(Debug, Serialize, Deserialize, Zeroize)]
pub(crate) struct EncryptedData<T> {
  #[serde(with = "base64")]
  ciphertext: Vec<u8>,
  #[serde(with = "base64")]
  salt: Vec<u8>,
  #[serde(with = "base64")]
  iv: Vec<u8>,

  #[serde(skip)]
  #[zeroize(skip)]
  _marker: PhantomData<T>,
}

impl<T> Drop for EncryptedData<T> {
  fn drop(&mut self) {
    self.zeroize()
  }
}

// we only ever want to expose those getters in the test code
#[cfg(test)]
impl<T> EncryptedData<T> {
  pub(crate) fn ciphertext(&self) -> &[u8] {
    &self.ciphertext
  }

  pub(crate) fn salt(&self) -> &[u8] {
    &self.salt
  }

  pub(crate) fn iv(&self) -> &[u8] {
    &self.iv
  }
}

// helper to make Vec<u8> serialization use base64 representation to make it human readable
// so that it would be easier for users to copy contents from the disk if they wanted to use it elsewhere
mod base64 {
  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::encode(bytes))
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let s = <String>::deserialize(deserializer)?;
    base64::decode(&s).map_err(serde::de::Error::custom)
  }
}

impl<T> EncryptedData<T> {
  pub(crate) fn encrypt_struct(data: &T, password: &UserPassword) -> Result<Self, BackendError>
  where
    T: Serialize,
  {
    encrypt_struct(data, password)
  }

  pub(crate) fn decrypt_struct(&self, password: &UserPassword) -> Result<T, BackendError>
  where
    T: for<'a> Deserialize<'a>,
  {
    decrypt_struct(self, password)
  }
}

impl EncryptedData<Vec<u8>> {
  pub(crate) fn encrypt_data(data: &[u8], password: &UserPassword) -> Result<Self, BackendError> {
    encrypt_data(data, password)
  }

  pub(crate) fn decrypt_data(&self, password: &UserPassword) -> Result<Vec<u8>, BackendError> {
    decrypt_data(self, password)
  }
}

fn derive_cipher_key<KeySize>(
  password: &UserPassword,
  salt: &[u8],
) -> Result<Key<KeySize>, BackendError>
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

fn random_salt_and_iv() -> (Vec<u8>, Vec<u8>) {
  let mut rng = OsRng;

  let mut salt = vec![0u8; SALT_LEN];
  rng.fill_bytes(&mut salt);

  let mut iv = vec![0u8; IV_LEN];
  rng.fill_bytes(&mut iv);

  (salt, iv)
}

fn encrypt(
  data: &[u8],
  password: &UserPassword,
  salt: &[u8],
  iv: &[u8],
) -> Result<Vec<u8>, BackendError> {
  let key = derive_cipher_key(password, salt)?;
  let cipher = Aes256Gcm::new(&key);
  cipher
    .encrypt(Nonce::from_slice(iv), data)
    .map_err(|_| BackendError::EncryptionError)
}

fn decrypt(
  ciphertext: &[u8],
  password: &UserPassword,
  salt: &[u8],
  iv: &[u8],
) -> Result<Vec<u8>, BackendError> {
  let key = derive_cipher_key(password, salt)?;
  let cipher = Aes256Gcm::new(&key);
  cipher
    .decrypt(Nonce::from_slice(iv), ciphertext)
    .map_err(|_| BackendError::DecryptionError)
}

pub(crate) fn encrypt_data(
  data: &[u8],
  password: &UserPassword,
) -> Result<EncryptedData<Vec<u8>>, BackendError> {
  let (salt, iv) = random_salt_and_iv();
  let ciphertext = encrypt(data, password, &salt, &iv)?;

  Ok(EncryptedData {
    ciphertext,
    salt,
    iv,
    _marker: Default::default(),
  })
}

pub(crate) fn encrypt_struct<T>(
  data: &T,
  password: &UserPassword,
) -> Result<EncryptedData<T>, BackendError>
where
  T: Serialize,
{
  let bytes = serde_json::to_vec(data).map_err(|_| BackendError::EncryptionError)?;

  let (salt, iv) = random_salt_and_iv();
  let ciphertext = encrypt(&bytes, password, &salt, &iv)?;

  Ok(EncryptedData {
    ciphertext,
    salt,
    iv,
    _marker: Default::default(),
  })
}

pub(crate) fn decrypt_data(
  encrypted_data: &EncryptedData<Vec<u8>>,
  password: &UserPassword,
) -> Result<Vec<u8>, BackendError> {
  decrypt(
    &encrypted_data.ciphertext,
    password,
    &encrypted_data.salt,
    &encrypted_data.iv,
  )
}

pub(crate) fn decrypt_struct<T>(
  encrypted_data: &EncryptedData<T>,
  password: &UserPassword,
) -> Result<T, BackendError>
where
  T: for<'a> Deserialize<'a>,
{
  let bytes = decrypt(
    &encrypted_data.ciphertext,
    password,
    &encrypted_data.salt,
    &encrypted_data.iv,
  )?;

  serde_json::from_slice(&bytes).map_err(|_| BackendError::DecryptionError)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Serialize, Deserialize, PartialEq, Debug)]
  struct DummyData {
    foo: String,
    bar: String,
  }

  #[test]
  fn struct_encryption() {
    let password = UserPassword::new("my-super-secret-password".to_string());
    let data = DummyData {
      foo: "my secret mnemonic".to_string(),
      bar: "totally-valid-hd-path".to_string(),
    };

    let wrong_password = UserPassword::new("brute-force-attempt-1".to_string());

    let mut encrypted_data = encrypt_struct(&data, &password).unwrap();
    let recovered = decrypt_struct(&encrypted_data, &password).unwrap();
    assert_eq!(data, recovered);

    // decryption with wrong password fails
    assert!(decrypt_struct(&encrypted_data, &wrong_password).is_err());

    // decryption fails if ciphertext got malformed
    encrypted_data.ciphertext[3] ^= 123;
    assert!(decrypt_struct(&encrypted_data, &wrong_password).is_err());

    // restore the ciphertext (for test purposes)
    encrypted_data.ciphertext[3] ^= 123;

    // decryption fails if salt got malformed (it would result in incorrect key being derived)
    encrypted_data.salt[3] ^= 123;
    assert!(decrypt_struct(&encrypted_data, &password).is_err());

    // restore the salt (for test purposes)
    encrypted_data.salt[3] ^= 123;

    // decryption fails if iv got malformed
    encrypted_data.iv[3] ^= 123;
    assert!(decrypt_struct(&encrypted_data, &password).is_err());
  }
}
