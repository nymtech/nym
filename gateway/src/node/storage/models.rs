// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::AvailableBandwidth;
use sqlx::FromRow;
use time::OffsetDateTime;

pub struct PersistedSharedKeys {
    pub(crate) client_address_bs58: String,
    pub(crate) derived_aes128_ctr_blake3_hmac_keys_bs58: String,
}

pub struct StoredMessage {
    pub(crate) id: i64,
    #[allow(dead_code)]
    pub(crate) client_address_bs58: String,
    pub(crate) content: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PersistedBandwidth {
    #[allow(dead_code)]
    pub(crate) client_address_bs58: String,
    pub(crate) available: i64,
    pub(crate) expiration: Option<OffsetDateTime>,
}

impl From<PersistedBandwidth> for AvailableBandwidth {
    fn from(value: PersistedBandwidth) -> Self {
        AvailableBandwidth {
            bytes: value.available,
            expiration: value.expiration.unwrap_or(OffsetDateTime::UNIX_EPOCH),
        }
    }
}

impl From<Option<PersistedBandwidth>> for AvailableBandwidth {
    fn from(value: Option<PersistedBandwidth>) -> Self {
        match value {
            None => AvailableBandwidth::default(),
            Some(b) => b.into(),
        }
    }
}

pub(crate) struct PendingStoredCredential {
    pub(crate) id: i64,
    pub(crate) credential: String,
    pub(crate) gateway_address: String,
    pub(crate) api_urls: String,
    pub(crate) proposal_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SpentCredential {
    #[allow(dead_code)]
    pub(crate) blinded_serial_number_bs58: String,
    #[allow(dead_code)]
    pub(crate) was_freepass: bool,
    #[allow(dead_code)]
    pub(crate) client_address_bs58: String,
}
