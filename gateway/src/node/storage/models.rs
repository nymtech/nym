// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::ecash::ClientTicket;
use crate::node::client_handling::websocket::connection_handler::AvailableBandwidth;
use crate::node::storage::error::StorageError;
use nym_credentials_interface::CredentialSpendingData;
use sqlx::FromRow;
use time::OffsetDateTime;

pub struct PersistedSharedKeys {
    #[allow(dead_code)]
    pub(crate) id: i64,

    #[allow(dead_code)]
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

#[derive(FromRow)]
pub struct VerifiedTicket {
    pub(crate) serial_number: Vec<u8>,
    pub(crate) ticket_id: i64,
}

#[derive(FromRow)]
pub struct RedemptionProposal {
    pub(crate) proposal_id: i64,
    pub(crate) created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub struct UnverifiedTicketData {
    pub(crate) data: Vec<u8>,
    pub(crate) ticket_id: i64,
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
