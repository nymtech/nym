// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::keys::KeyPairWithEpoch;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::{error::CompactEcashError, scheme::keygen::SecretKeyAuth, KeyPairAuth};
use nym_pemstore::traits::PemStorableKey;
use std::mem;

impl PemStorableKey for KeyPairWithEpoch {
    // that's not the best error for this, but it felt like an overkill to define a dedicated struct just for this purpose
    type Error = CompactEcashError;

    fn pem_type() -> &'static str {
        "COCONUT KEY WITH EPOCH" // avoid the invalidation of already present key
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.issued_for_epoch.to_be_bytes().to_vec();
        bytes.append(&mut self.keys.secret_key().to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() <= mem::size_of::<EpochId>() {
            return Err(CompactEcashError::DeserializationMinLength {
                min: mem::size_of::<EpochId>(),
                actual: bytes.len(),
            });
        }
        let epoch_id = EpochId::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        let sk = SecretKeyAuth::from_bytes(&bytes[mem::size_of::<EpochId>()..])?;
        let vk = sk.verification_key();

        Ok(KeyPairWithEpoch {
            keys: KeyPairAuth::from_keys(sk, vk),
            issued_for_epoch: epoch_id,
        })
    }
}
