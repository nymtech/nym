// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_compact_ecash::setup::GroupParameters;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;

use nym_crypto::asymmetric::{encryption, identity};

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

pub struct State {
    pub voucher: BandwidthVoucher,
    pub params: GroupParameters,
}

impl State {
    pub fn new(voucher: BandwidthVoucher) -> Self {
        State {
            voucher,
            params: GroupParameters::new().unwrap(),
        }
    }
}
