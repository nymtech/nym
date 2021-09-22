// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) struct PersistedSharedKeys {
    pub(crate) client_address_bs58: String,
    pub(crate) derived_aes128_ctr_blake3_hmac_keys_bs58: String,
}

pub(crate) struct StoredMessage {
    pub(crate) id: i64,
    pub(crate) client_address_bs58: String,
    pub(crate) content: Vec<u8>,
}

pub(crate) struct PersistedBandwidth {
    pub(crate) client_address_bs58: String,
    pub(crate) available: i64,
}
