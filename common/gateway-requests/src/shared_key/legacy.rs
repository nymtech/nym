// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::KDF_SALT_LENGTH;
use crate::shared_key::SharedSymmetricKey;
use crate::shared_key::{SharedKeyConversionError, SharedKeySize, SharedKeyUsageError};
use crate::LegacyGatewayMacSize;
use nym_crypto::generic_array::{
    typenum::{Sum, Unsigned, U16},
    GenericArray,
};
use nym_crypto::hkdf;
use nym_crypto::hmac::{compute_keyed_hmac, recompute_keyed_hmac_and_verify_tag};
use nym_crypto::symmetric::stream_cipher::{self, CipherKey, KeySizeUser, IV};
use nym_pemstore::traits::PemStorableKey;
use nym_sphinx::params::{
    GatewayIntegrityHmacAlgorithm, GatewaySharedKeyHkdfAlgorithm, LegacyGatewayEncryptionAlgorithm,
};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

// shared key is as long as the encryption key and the MAC key combined.
pub type LegacySharedKeySize = Sum<EncryptionKeySize, MacKeySize>;

// we're using 16 byte long key in sphinx, so let's use the same one here
type MacKeySize = U16;
type EncryptionKeySize = <LegacyGatewayEncryptionAlgorithm as KeySizeUser>::KeySize;

/// Shared key used when computing MAC for messages exchanged between client and its gateway.
pub type MacKey = GenericArray<u8, MacKeySize>;

#[derive(Debug, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct LegacySharedKeys {
    encryption_key: CipherKey<LegacyGatewayEncryptionAlgorithm>,
    mac_key: MacKey,
}

impl LegacySharedKeys {
    pub fn upgrade(&self) -> (SharedSymmetricKey, Vec<u8>) {
        let mut rng = thread_rng();
        let mut salt = vec![0u8; KDF_SALT_LENGTH];
        rng.fill_bytes(&mut salt);

        let legacy_bytes = Zeroizing::new(self.to_bytes());
        let okm = hkdf::extract_then_expand::<GatewaySharedKeyHkdfAlgorithm>(
            Some(&salt),
            &legacy_bytes,
            None,
            SharedKeySize::to_usize(),
        )
        .expect("somehow too long okm was provided");

        let key = SharedSymmetricKey::try_from_bytes(&okm)
            .expect("okm was expanded to incorrect length!");
        (key, salt)
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, SharedKeyConversionError> {
        if bytes.len() != LegacySharedKeySize::to_usize() {
            return Err(SharedKeyConversionError::InvalidSharedKeysSize {
                received: bytes.len(),
                expected: LegacySharedKeySize::to_usize(),
            });
        }

        let encryption_key =
            GenericArray::clone_from_slice(&bytes[..EncryptionKeySize::to_usize()]);
        let mac_key = GenericArray::clone_from_slice(&bytes[EncryptionKeySize::to_usize()..]);

        Ok(LegacySharedKeys {
            encryption_key,
            mac_key,
        })
    }

    /// Encrypts the provided data using the optionally provided initialisation vector,
    /// or a 0 value if nothing was given.
    /// It does **NOT** attach any integrity macs on the produced ciphertext
    pub fn encrypt_without_tagging(
        &self,
        data: &[u8],
        iv: Option<&IV<LegacyGatewayEncryptionAlgorithm>>,
    ) -> Vec<u8> {
        match iv {
            Some(iv) => stream_cipher::encrypt::<LegacyGatewayEncryptionAlgorithm>(
                self.encryption_key(),
                iv,
                data,
            ),
            None => {
                let zero_iv = stream_cipher::zero_iv::<LegacyGatewayEncryptionAlgorithm>();
                stream_cipher::encrypt::<LegacyGatewayEncryptionAlgorithm>(
                    self.encryption_key(),
                    &zero_iv,
                    data,
                )
            }
        }
    }

    /// Encrypts the provided data using the optionally provided initialisation vector,
    /// or a 0 value if nothing was given. Then it computes an integrity mac and concatenates it
    /// with the previously produced ciphertext.
    pub fn encrypt_and_tag(
        &self,
        data: &[u8],
        iv: Option<&IV<LegacyGatewayEncryptionAlgorithm>>,
    ) -> Vec<u8> {
        let ciphertext = self.encrypt_without_tagging(data, iv);
        let mac = compute_keyed_hmac::<GatewayIntegrityHmacAlgorithm>(
            self.mac_key().as_slice(),
            &ciphertext,
        );

        mac.into_bytes().into_iter().chain(ciphertext).collect()
    }

    pub fn decrypt_without_tag(
        &self,
        ciphertext: &[u8],
        iv: Option<&IV<LegacyGatewayEncryptionAlgorithm>>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        let zero_iv = stream_cipher::zero_iv::<LegacyGatewayEncryptionAlgorithm>();
        let iv = iv.unwrap_or(&zero_iv);
        Ok(stream_cipher::decrypt::<LegacyGatewayEncryptionAlgorithm>(
            self.encryption_key(),
            iv,
            ciphertext,
        ))
    }

    pub fn decrypt_tagged(
        &self,
        enc_data: &[u8],
        iv: Option<&IV<LegacyGatewayEncryptionAlgorithm>>,
    ) -> Result<Vec<u8>, SharedKeyUsageError> {
        let mac_size = LegacyGatewayMacSize::to_usize();
        if enc_data.len() < mac_size {
            return Err(SharedKeyUsageError::TooShortRequest);
        }

        let mac_tag = &enc_data[..mac_size];
        let message_bytes = &enc_data[mac_size..];

        if !recompute_keyed_hmac_and_verify_tag::<GatewayIntegrityHmacAlgorithm>(
            self.mac_key().as_slice(),
            message_bytes,
            mac_tag,
        ) {
            return Err(SharedKeyUsageError::InvalidMac);
        }

        // couldn't have made the first borrow mutable as you can't have an immutable borrow
        // together with a mutable one
        let mut message_bytes_mut = message_bytes.to_vec();

        let zero_iv = stream_cipher::zero_iv::<LegacyGatewayEncryptionAlgorithm>();
        let iv = iv.unwrap_or(&zero_iv);
        stream_cipher::decrypt_in_place::<LegacyGatewayEncryptionAlgorithm>(
            self.encryption_key(),
            iv,
            &mut message_bytes_mut,
        );
        Ok(message_bytes_mut)
    }

    pub fn encryption_key(&self) -> &CipherKey<LegacyGatewayEncryptionAlgorithm> {
        &self.encryption_key
    }

    pub fn mac_key(&self) -> &MacKey {
        &self.mac_key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.encryption_key
            .iter()
            .copied()
            .chain(self.mac_key.iter().copied())
            .collect()
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, SharedKeyConversionError> {
        let decoded = bs58::decode(val.into()).into_vec()?;
        LegacySharedKeys::try_from_bytes(&decoded)
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }
}

impl From<LegacySharedKeys> for String {
    fn from(keys: LegacySharedKeys) -> Self {
        keys.to_base58_string()
    }
}

impl PemStorableKey for LegacySharedKeys {
    type Error = SharedKeyConversionError;

    fn pem_type() -> &'static str {
        // TODO: If common\nymsphinx\params\src\lib::GatewayIntegrityHmacAlgorithm changes
        // the pem type needs updating!
        "AES-128-CTR + HMAC-BLAKE3 GATEWAY SHARED KEYS"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}
