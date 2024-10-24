// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{LegacySharedKeys, SharedGatewayKey, SharedKeyUsageError, SharedSymmetricKey};
use nym_crypto::symmetric::aead::random_nonce;
use nym_crypto::symmetric::stream_cipher::random_iv;
use nym_sphinx::params::{GatewayEncryptionAlgorithm, LegacyGatewayEncryptionAlgorithm};
use rand::thread_rng;

pub trait SymmetricKey {
    fn random_nonce_or_iv(&self) -> Vec<u8>;

    fn encrypt(
        &self,
        plaintext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError>;

    fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError>;
}

impl SymmetricKey for SharedGatewayKey {
    fn random_nonce_or_iv(&self) -> Vec<u8> {
        self.random_nonce_or_iv()
    }

    fn encrypt(
        &self,
        plaintext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        self.encrypt(plaintext, nonce)
    }

    fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        self.decrypt(ciphertext, nonce)
    }
}

impl SymmetricKey for SharedSymmetricKey {
    fn random_nonce_or_iv(&self) -> Vec<u8> {
        let mut rng = thread_rng();

        random_nonce::<GatewayEncryptionAlgorithm, _>(&mut rng).to_vec()
    }

    fn encrypt(
        &self,
        plaintext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        let nonce = SharedGatewayKey::validate_aead_nonce(nonce)?;
        self.encrypt(plaintext, &nonce)
    }

    fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        let nonce = SharedGatewayKey::validate_aead_nonce(nonce)?;
        self.decrypt(ciphertext, &nonce)
    }
}

impl SymmetricKey for LegacySharedKeys {
    fn random_nonce_or_iv(&self) -> Vec<u8> {
        let mut rng = thread_rng();

        random_iv::<LegacyGatewayEncryptionAlgorithm, _>(&mut rng).to_vec()
    }

    fn encrypt(
        &self,
        plaintext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        let iv = SharedGatewayKey::validate_cipher_iv(nonce)?;
        Ok(self.encrypt_and_tag(plaintext, iv))
    }

    fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: Option<&[u8]>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        let iv = SharedGatewayKey::validate_cipher_iv(nonce)?;
        self.decrypt_tagged(ciphertext, iv)
    }
}
