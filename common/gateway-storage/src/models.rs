// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::StorageError;
use nym_credentials_interface::{AvailableBandwidth, ClientTicket, CredentialSpendingData};
use sqlx::{types::chrono::NaiveDateTime, FromRow};
use time::OffsetDateTime;

pub struct PersistedSharedKeys {
    #[allow(dead_code)]
    pub id: i64,

    #[allow(dead_code)]
    pub client_address_bs58: String,
    pub derived_aes128_ctr_blake3_hmac_keys_bs58: String,
}

pub struct StoredMessage {
    pub id: i64,
    #[allow(dead_code)]
    pub client_address_bs58: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PersistedBandwidth {
    #[allow(dead_code)]
    pub client_id: i64,
    pub available: i64,
    pub expiration: Option<OffsetDateTime>,
}

impl From<PersistedBandwidth> for AvailableBandwidth {
    fn from(value: PersistedBandwidth) -> Self {
        AvailableBandwidth {
            bytes: value.available,
            expiration: value.expiration.unwrap_or(OffsetDateTime::UNIX_EPOCH),
        }
    }
}

#[derive(FromRow)]
pub struct VerifiedTicket {
    pub serial_number: Vec<u8>,
    pub ticket_id: i64,
}

#[derive(FromRow)]
pub struct RedemptionProposal {
    pub proposal_id: i64,
    pub created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub struct UnverifiedTicketData {
    pub data: Vec<u8>,
    pub ticket_id: i64,
}

impl TryFrom<UnverifiedTicketData> for ClientTicket {
    type Error = StorageError;

    fn try_from(value: UnverifiedTicketData) -> Result<Self, Self::Error> {
        Ok(ClientTicket {
            spending_data: CredentialSpendingData::try_from_bytes(&value.data).map_err(|_| {
                StorageError::MalformedStoredTicketData {
                    ticket_id: value.ticket_id,
                }
            })?,
            ticket_id: value.ticket_id,
        })
    }
}

#[cfg(feature = "wireguard")]
#[derive(Debug, Clone, FromRow)]
pub struct WireguardPeer {
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub protocol_version: Option<i64>,
    pub endpoint: Option<String>,
    pub last_handshake: Option<sqlx::types::chrono::NaiveDateTime>,
    pub tx_bytes: i64,
    pub rx_bytes: i64,
    pub persistent_keepalive_interval: Option<i64>,
    pub allowed_ips: Vec<u8>,
    pub suspended: bool,
}
