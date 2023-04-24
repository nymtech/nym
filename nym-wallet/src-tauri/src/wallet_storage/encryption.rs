// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::password::UserPassword;
use crate::error::BackendError;
use bip39::rand_core::OsRng;
use nym_store_cipher::{
    Aes256Gcm, Algorithm, EncryptedData as StoreEncryptedData, KdfInfo, Params, StoreCipher,
    Version, CURRENT_VERSION,
};
use serde::{Deserialize, Serialize};
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
        self.zeroize();
    }
}

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
        base64::decode(s).map_err(serde::de::Error::custom)
    }
}

impl<T> EncryptedData<T> {
    pub(crate) fn decrypt_struct(&self, password: &UserPassword) -> Result<T, BackendError>
    where
        T: for<'a> Deserialize<'a>,
    {
        decrypt_struct(self, password)
    }
}

fn instantiate_cipher_store(
    password: &UserPassword,
    salt: &[u8],
) -> Result<StoreCipher<Aes256Gcm>, BackendError> {
    let mut kdf_salt: [u8; SALT_LEN] = Default::default();
    kdf_salt.copy_from_slice(salt);

    // use the same parameters as we did in the past
    let kdf_info = KdfInfo::Argon2 {
        params: Params::new(MEMORY_COST, ITERATIONS, PARALLELISM, Some(OUTPUT_LENGTH)).unwrap(),
        algorithm: Algorithm::Argon2id,
        version: Version::V0x13,
        kdf_salt,
    };

    Ok(StoreCipher::new(password.as_ref(), kdf_info)?)
}

/// Wraps `ciphertext` and `iv` into `[nym_store_cipher::StoreEncryptedData]`
fn new_store_encrypted_data(ciphertext: &[u8], iv: &[u8]) -> StoreEncryptedData {
    StoreEncryptedData {
        // well, we can only assume the current version
        version: CURRENT_VERSION,
        ciphertext: ciphertext.to_owned(),
        nonce: iv.to_owned(),
    }
}

pub(crate) fn encrypt_struct<T>(
    data: &T,
    password: &UserPassword,
) -> Result<EncryptedData<T>, BackendError>
where
    T: Serialize,
{
    let mut rng = OsRng;
    let salt = KdfInfo::random_salt_with_rng(&mut rng)?;

    let cipher = instantiate_cipher_store(password, &salt)?;
    let ciphertext = cipher.encrypt_json_value(data)?;
    assert_eq!(ciphertext.nonce.len(), IV_LEN);

    Ok(EncryptedData {
        ciphertext: ciphertext.ciphertext,
        salt: salt.to_vec(),
        iv: ciphertext.nonce,
        _marker: Default::default(),
    })
}

pub(crate) fn decrypt_struct<T>(
    encrypted_data: &EncryptedData<T>,
    password: &UserPassword,
) -> Result<T, BackendError>
where
    T: for<'a> Deserialize<'a>,
{
    let cipher = instantiate_cipher_store(password, encrypted_data.salt())?;
    Ok(cipher.decrypt_json_value(new_store_encrypted_data(
        encrypted_data.ciphertext(),
        encrypted_data.iv(),
    ))?)
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
