// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use aes_gcm::aead::{Aead, Nonce};
use aes_gcm::{AeadCore, AeadInPlace, KeyInit};
use rand::{thread_rng, CryptoRng, Fill, RngCore};
use serde::{Deserialize, Serialize};
use serde_helpers::{argon2_algorithm_helper, argon2_params_helper, argon2_version_helper};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub use aes_gcm::Aes256Gcm;
pub use aes_gcm::{Key, KeySizeUser};
pub use argon2::{Algorithm, Argon2, Params, Version};
pub use generic_array::typenum::Unsigned;

mod serde_helpers;

pub const CURRENT_VERSION: u8 = 1;
pub const ARGON2_SALT_SIZE: usize = 16;
pub const AES256GCM_NONCE_SIZE: usize = 12;

const VERIFICATION_PHRASE: &[u8] = &[0u8; 32];

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unsupported cipher")]
    UnsupportedCipher,

    #[error("failed to encrypt/decrypt provided data: {cause}")]
    AesFailure { cause: aes_gcm::Error },

    #[error("failed to expand the passphrase: {cause}")]
    Argon2Failure { cause: argon2::Error },

    #[cfg(feature = "json")]
    #[error("failed to serialize/deserialize JSON: {source}")]
    SerdeJsonFailure {
        #[from]
        source: serde_json::Error,
    },

    #[error("failed to generate random bytes: {source}")]
    RandomError {
        #[from]
        source: rand::Error,
    },

    #[error("the received ciphertext was encrypted with different store version ({received}). The current version is {CURRENT_VERSION}")]
    VersionMismatch { received: u8 },

    #[error("the decoded verification phrase did not match the expected value")]
    VerificationPhraseMismatch,

    #[error("could not import the store - the provided passphrase was invalid")]
    InvalidImportPassphrase,
}

// it's weird that this couldn't be auto-derived with a `#[from]`...
impl From<aes_gcm::Error> for Error {
    fn from(cause: aes_gcm::Error) -> Self {
        Error::AesFailure { cause }
    }
}

impl From<argon2::Error> for Error {
    fn from(cause: argon2::Error) -> Self {
        Error::Argon2Failure { cause }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KdfInfo {
    Argon2 {
        /// The Argon2 parameters that were used when deriving the store key.
        #[serde(with = "argon2_params_helper")]
        params: Params,

        /// The specific Argon2 algorithm variant used when deriving the store key.
        #[serde(with = "argon2_algorithm_helper")]
        algorithm: Algorithm,

        /// The specific version of the Argon2 algorithm used when deriving the store key.
        #[serde(with = "argon2_version_helper")]
        version: Version,

        /// The salt that was used when the passphrase was expanded into a store key.
        kdf_salt: [u8; ARGON2_SALT_SIZE],
    },
}

impl KdfInfo {
    pub fn expand_key<C>(&self, passphrase: &[u8]) -> Result<Key<C>, Error>
    where
        C: KeySizeUser,
    {
        match self {
            KdfInfo::Argon2 {
                params,
                algorithm,
                version,
                kdf_salt,
            } => argon2_derive_cipher_key::<C>(
                passphrase,
                kdf_salt,
                &[],
                params.clone(),
                *algorithm,
                *version,
            ),
        }
    }

    pub fn new_with_default_settings() -> Result<Self, Error> {
        let kdf_salt = Self::random_salt()?;
        Ok(KdfInfo::Argon2 {
            params: Default::default(),
            algorithm: Default::default(),
            version: Default::default(),
            kdf_salt,
        })
    }

    pub fn random_salt() -> Result<[u8; ARGON2_SALT_SIZE], Error> {
        let mut rng = thread_rng();
        Self::random_salt_with_rng(&mut rng)
    }

    pub fn random_salt_with_rng<R: RngCore + CryptoRng>(
        rng: &mut R,
    ) -> Result<[u8; ARGON2_SALT_SIZE], Error> {
        let mut salt = [0u8; ARGON2_SALT_SIZE];
        salt.try_fill(rng)?;
        Ok(salt)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CiphertextInfo {
    Aes256Gcm {
        /// The nonce that was used to encrypt the ciphertext.
        nonce: [u8; AES256GCM_NONCE_SIZE],
        ciphertext: Vec<u8>,
    },
}

impl CiphertextInfo {
    pub fn nonce<C>(&self) -> &Nonce<C>
    where
        C: AeadCore,
    {
        match self {
            CiphertextInfo::Aes256Gcm { nonce, .. } => Nonce::<C>::from_slice(nonce),
        }
    }

    pub fn ciphertext(&self) -> &[u8] {
        match self {
            CiphertextInfo::Aes256Gcm { ciphertext, .. } => ciphertext,
        }
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct StoreCipher<C = Aes256Gcm>
where
    C: KeySizeUser,
{
    key: Key<C>,

    #[zeroize(skip)]
    kdf_info: KdfInfo,
}

impl StoreCipher<Aes256Gcm> {
    pub fn import_aes256gcm(
        passphrase: &[u8],
        exported: ExportedStoreCipher,
    ) -> Result<Self, Error> {
        // that's a terrible interface, but we can refactor it later
        if !matches!(exported.ciphertext_info, CiphertextInfo::Aes256Gcm { .. }) {
            return Err(Error::UnsupportedCipher);
        }

        let mut key = exported.kdf_info.expand_key::<Aes256Gcm>(passphrase)?;

        // check if correct key was derived
        let Ok(plaintext) = Aes256Gcm::new(&key).decrypt(
            exported.ciphertext_info.nonce::<Aes256Gcm>(),
            exported.ciphertext_info.ciphertext(),
        ) else {
            key.zeroize();
            return Err(Error::InvalidImportPassphrase);
        };

        // if we successfully decrypted aes256gcm ciphertext, it's almost certainly correct
        // otherwise the tag wouldn't match, but let's do the sanity sake
        if plaintext != VERIFICATION_PHRASE {
            key.zeroize();
            return Err(Error::VerificationPhraseMismatch);
        }

        Ok(StoreCipher {
            key,
            kdf_info: exported.kdf_info,
        })
    }

    pub fn export_aes256gcm(&self) -> Result<ExportedStoreCipher, Error> {
        let verification_ciphertext = self.encrypt_data_ref(VERIFICATION_PHRASE)?;

        Ok(ExportedStoreCipher {
            kdf_info: self.kdf_info.clone(),
            ciphertext_info: CiphertextInfo::Aes256Gcm {
                // the unwrap is fine, otherwise it implies we've been using incorrect nonces all along!
                nonce: verification_ciphertext.nonce.try_into().unwrap(),
                ciphertext: verification_ciphertext.ciphertext,
            },
        })
    }

    pub fn new_aes256gcm(passphrase: &[u8], kdf_info: KdfInfo) -> Result<Self, Error> {
        Self::new(passphrase, kdf_info)
    }
}

impl<C: KeySizeUser + KeyInit> StoreCipher<C>
where
    C: KeySizeUser + KeyInit,
{
    pub fn new(passphrase: &[u8], kdf_info: KdfInfo) -> Result<Self, Error> {
        let key = kdf_info.expand_key::<C>(passphrase)?;
        Ok(StoreCipher { key, kdf_info })
    }

    pub fn new_with_default_kdf(passphrase: &[u8]) -> Result<Self, Error> {
        let kdf_info = KdfInfo::new_with_default_settings()?;
        Self::new(passphrase, kdf_info)
    }

    #[cfg(feature = "json")]
    pub fn encrypt_json_value<T: Serialize>(&self, data: &T) -> Result<EncryptedData, Error>
    where
        C: AeadInPlace,
    {
        let raw = serde_json::to_vec(data)?;
        self.encrypt_data(raw)
    }

    // Unless you know what you're doing, use `Self::encrypt_data` instead.
    // As the caller of this method needs to make sure to correctly dispose of the original plaintext.
    pub fn encrypt_data_ref(&self, data: &[u8]) -> Result<EncryptedData, Error>
    where
        C: Aead,
    {
        let nonce = Self::random_nonce()?;

        let cipher = C::new(&self.key);
        let ciphertext = cipher.encrypt(&nonce, data)?;

        Ok(EncryptedData {
            version: CURRENT_VERSION,
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    pub fn encrypt_data(&self, mut data: Vec<u8>) -> Result<EncryptedData, Error>
    where
        C: AeadInPlace,
    {
        let nonce = Self::random_nonce()?;

        let cipher = C::new(&self.key);
        cipher.encrypt_in_place(&nonce, &[], &mut data)?;

        Ok(EncryptedData {
            version: CURRENT_VERSION,
            ciphertext: data,
            nonce: nonce.to_vec(),
        })
    }

    #[cfg(feature = "json")]
    pub fn decrypt_json_value<T: serde::de::DeserializeOwned>(
        &self,
        data: EncryptedData,
    ) -> Result<T, Error>
    where
        C: AeadInPlace,
    {
        let plaintext = zeroize::Zeroizing::new(self.decrypt_data(data)?);
        let value = serde_json::from_slice(&plaintext)?;
        Ok(value)
    }

    pub fn decrypt_data_unchecked(&self, data: EncryptedData) -> Result<Vec<u8>, Error>
    where
        C: Aead,
    {
        let cipher = C::new(&self.key);
        let plaintext = cipher.decrypt(
            Nonce::<C>::from_slice(&data.nonce),
            data.ciphertext.as_ref(),
        )?;
        Ok(plaintext)
    }

    pub fn decrypt_data(&self, data: EncryptedData) -> Result<Vec<u8>, Error>
    where
        C: Aead,
    {
        if data.version != CURRENT_VERSION {
            return Err(Error::VersionMismatch {
                received: data.version,
            });
        }

        self.decrypt_data_unchecked(data)
    }

    pub fn random_nonce() -> Result<Nonce<C>, Error>
    where
        C: AeadCore,
    {
        let mut rng = thread_rng();
        Self::random_nonce_with_rng(&mut rng)
    }

    pub fn random_nonce_with_rng<R: RngCore + CryptoRng>(rng: &mut R) -> Result<Nonce<C>, Error>
    where
        C: AeadCore,
    {
        let mut nonce = Nonce::<C>::default();
        nonce.try_fill(rng)?;
        Ok(nonce)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedStoreCipher {
    /// Info about the key derivation method that was used to expand the
    /// passphrase into an encryption key.
    pub kdf_info: KdfInfo,

    /// The ciphertext of known plaintext and additional data that is needed to
    /// verify correct key derivation and cipher choice.
    pub ciphertext_info: CiphertextInfo,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct EncryptedData {
    pub version: u8,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub fn argon2_derive_cipher_key<C>(
    passphrase: &[u8],
    salt: &[u8],
    pepper: &[u8],
    params: Params,
    algorithm: Algorithm,
    version: Version,
) -> Result<Key<C>, Error>
where
    C: KeySizeUser,
{
    let argon2 = if pepper.is_empty() {
        Argon2::new(algorithm, version, params)
    } else {
        Argon2::new_with_secret(pepper, algorithm, version, params)?
    };

    let mut key = Key::<C>::default();
    argon2.hash_password_into(passphrase, salt, &mut key)?;

    Ok(key)
}
