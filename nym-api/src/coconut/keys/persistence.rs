// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::keys::KeyPairWithEpoch;
use crate::coconut::state::BANDWIDTH_CREDENTIAL_PARAMS;
use nym_coconut::{CoconutError, KeyPair, SecretKey};
use nym_coconut_dkg_common::types::EpochId;
use nym_pemstore::traits::PemStorableKey;
use std::mem;

impl PemStorableKey for KeyPairWithEpoch {
    // that's not the best error for this, but it felt like an overkill to define a dedicated struct just for this purpose
    type Error = CoconutError;

    fn pem_type() -> &'static str {
        "COCONUT KEY WITH EPOCH"
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.issued_for_epoch.to_be_bytes().to_vec();
        bytes.append(&mut self.keys.secret_key().to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() <= mem::size_of::<EpochId>() {
            return Err(CoconutError::Deserialization(
                "insufficient number of bytes to decode secret key with epoch id".into(),
            ));
        }
        let epoch_id = EpochId::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        let sk = SecretKey::from_bytes(&bytes[mem::size_of::<EpochId>()..])?;
        let vk = sk.verification_key(&BANDWIDTH_CREDENTIAL_PARAMS);

        Ok(KeyPairWithEpoch {
            keys: KeyPair::from_keys(sk, vk),
            issued_for_epoch: epoch_id,
        })
    }
}
