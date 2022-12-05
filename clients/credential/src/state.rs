// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crypto::asymmetric::{encryption, identity};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct KeyPair {
    pub public_key: String,
    pub private_key: String,
}

impl From<identity::KeyPair> for KeyPair {
    fn from(kp: identity::KeyPair) -> Self {
        Self {
            public_key: kp.public_key().to_base58_string(),
            private_key: kp.private_key().to_base58_string(),
        }
    }
}

impl From<encryption::KeyPair> for KeyPair {
    fn from(kp: encryption::KeyPair) -> Self {
        Self {
            public_key: kp.public_key().to_base58_string(),
            private_key: kp.private_key().to_base58_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct State {
    pub amount: u64,
    pub tx_hash: String,
    pub signing_keypair: KeyPair,
    pub encryption_keypair: KeyPair,
}
