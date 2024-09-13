// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::error::HandshakeError;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::symmetric::aead::nonce_size;
use nym_sphinx::params::GatewayEncryptionAlgorithm;
use std::iter::once;

// it is vital nobody changes the serialisation implementation unless you have an EXTREMELY good reason,
// as otherwise you have very high chance of breaking backwards compatibility
pub trait HandshakeMessage {
    fn into_bytes(self) -> Vec<u8>;

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, HandshakeError>
    where
        Self: Sized;
}

pub struct Initialisation {
    pub identity: ed25519::PublicKey,
    pub ephemeral_dh: x25519::PublicKey,
    pub derive_aes256_gcm_siv_key: bool,
}

pub struct MaterialExchange {
    pub signature_ciphertext: [u8; ed25519::SIGNATURE_LENGTH],
    pub nonce: Option<Vec<u8>>,
}

impl MaterialExchange {
    pub fn attach_ephemeral_dh(self, ephemeral_dh: x25519::PublicKey) -> GatewayMaterialExchange {
        GatewayMaterialExchange {
            ephemeral_dh,
            materials: self,
        }
    }
}

pub struct GatewayMaterialExchange {
    pub ephemeral_dh: x25519::PublicKey,
    pub materials: MaterialExchange,
}

pub struct Finalization {
    pub success: bool,
}

impl Finalization {
    pub fn ensure_success(&self) -> Result<(), HandshakeError> {
        if !self.success {
            return Err(HandshakeError::HandshakeFailure);
        }
        Ok(())
    }
}

impl HandshakeMessage for Initialisation {
    // LOCAL_ID_PUBKEY || EPHEMERAL_KEY || MAYBE_NON_LEGACY
    // Eventually the ID_PUBKEY prefix will get removed and recipient will know
    // initializer's identity from another source.
    fn into_bytes(self) -> Vec<u8> {
        let bytes = self
            .identity
            .to_bytes()
            .into_iter()
            .chain(self.ephemeral_dh.to_bytes());

        if self.derive_aes256_gcm_siv_key {
            bytes.chain(once(1)).collect()
        } else {
            bytes.collect()
        }
    }

    // this will need to be adjusted when REMOTE_ID_PUBKEY is removed
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, HandshakeError>
    where
        Self: Sized,
    {
        let legacy_len = ed25519::PUBLIC_KEY_LENGTH + x25519::PUBLIC_KEY_SIZE;
        let current_len = legacy_len + 1;
        if bytes.len() != legacy_len && bytes.len() != current_len {
            return Err(HandshakeError::MalformedRequest);
        }

        let identity = ed25519::PublicKey::from_bytes(&bytes[..ed25519::PUBLIC_KEY_LENGTH])
            .map_err(|_| HandshakeError::MalformedRequest)?;

        // this can only fail if the provided bytes have len different from encryption::PUBLIC_KEY_SIZE
        // which is impossible
        let ephemeral_dh =
            x25519::PublicKey::from_bytes(&bytes[ed25519::PUBLIC_KEY_LENGTH..legacy_len]).unwrap();

        let derive_aes256_gcm_siv_key = if bytes.len() == legacy_len {
            false
        } else {
            if bytes[legacy_len] != 1 {
                return Err(HandshakeError::MalformedRequest);
            }
            true
        };

        Ok(Initialisation {
            identity,
            ephemeral_dh,
            derive_aes256_gcm_siv_key,
        })
    }
}

impl HandshakeMessage for MaterialExchange {
    // AES(k, SIG(PRIV_GATE, G^y || G^x))
    fn into_bytes(self) -> Vec<u8> {
        if let Some(nonce) = self.nonce {
            self.signature_ciphertext
                .iter()
                .cloned()
                .chain(nonce)
                .collect()
        } else {
            self.signature_ciphertext.iter().cloned().collect()
        }
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, HandshakeError>
    where
        Self: Sized,
    {
        // we expect to receive either:
        // LEGACY: ed25519 signature ciphertext (64 bytes)
        // CURRENT: ed25519 signature ciphertext + AES256-GCM-SIV nonce (76 bytes)
        let legacy_len = ed25519::SIGNATURE_LENGTH;
        let current_len = legacy_len + nonce_size::<GatewayEncryptionAlgorithm>();

        if bytes.len() != legacy_len && bytes.len() != current_len {
            return Err(HandshakeError::MalformedResponse);
        }

        let mut signature_ciphertext = [0u8; ed25519::SIGNATURE_LENGTH];
        signature_ciphertext.copy_from_slice(&bytes[..legacy_len]);

        let nonce = if bytes.len() == current_len {
            Some(bytes[legacy_len..].to_vec())
        } else {
            None
        };

        Ok(MaterialExchange {
            signature_ciphertext,
            nonce,
        })
    }
}

impl HandshakeMessage for GatewayMaterialExchange {
    // G^y || AES(k, SIG(PRIV_GATE, G^y || G^x))
    fn into_bytes(self) -> Vec<u8> {
        self.ephemeral_dh
            .to_bytes()
            .into_iter()
            .chain(self.materials.into_bytes().into_iter())
            .collect()
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, HandshakeError>
    where
        Self: Sized,
    {
        // we expect to receive either:
        // LEGACY: x25519 pubkey + ed25519 signature ciphertext (96 bytes)
        // CURRENT: x25519 pubkey + ed25519 signature ciphertext + AES256-GCM-SIV nonce (112 bytes)
        let legacy_len = x25519::PUBLIC_KEY_SIZE + ed25519::SIGNATURE_LENGTH;
        let current_len = legacy_len + nonce_size::<GatewayEncryptionAlgorithm>();

        if bytes.len() != legacy_len && bytes.len() != current_len {
            return Err(HandshakeError::MalformedResponse);
        }

        // this can only fail if the provided bytes have len different from PUBLIC_KEY_SIZE
        // which is impossible
        let ephemeral_dh =
            x25519::PublicKey::from_bytes(&bytes[..x25519::PUBLIC_KEY_SIZE]).unwrap();
        let materials = MaterialExchange::try_from_bytes(&bytes[x25519::PUBLIC_KEY_SIZE..])?;

        Ok(GatewayMaterialExchange {
            ephemeral_dh,
            materials,
        })
    }
}

impl HandshakeMessage for Finalization {
    fn into_bytes(self) -> Vec<u8> {
        if self.success {
            vec![1]
        } else {
            vec![0]
        }
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, HandshakeError>
    where
        Self: Sized,
    {
        if bytes.len() != 1 {
            return Err(HandshakeError::MalformedResponse);
        }

        let success = if bytes[0] == 1 { true } else { false };
        Ok(Finalization { success })
    }
}
