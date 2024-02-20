// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use sqlx::FromRow;

pub(crate) struct PersistedSharedKeys {
    pub(crate) client_address_bs58: String,
    pub(crate) derived_aes128_ctr_blake3_hmac_keys_bs58: String,
}

pub(crate) struct StoredMessage {
    pub(crate) id: i64,
    #[allow(dead_code)]
    pub(crate) client_address_bs58: String,
    pub(crate) content: Vec<u8>,
}

pub(crate) struct PersistedBandwidth {
    #[allow(dead_code)]
    pub(crate) client_address_bs58: String,
    pub(crate) available: i64,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct SpentCredential {
    #[allow(dead_code)]
    pub(crate) blinded_serial_number_bs58: String,
    #[allow(dead_code)]
    pub(crate) was_freepass: bool,
    #[allow(dead_code)]
    pub(crate) client_address_bs58: String,
}
