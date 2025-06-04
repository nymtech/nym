// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::GatewayStorageError;
use nym_credentials_interface::{AvailableBandwidth, ClientTicket, CredentialSpendingData};
use nym_gateway_requests::shared_key::{LegacySharedKeys, SharedGatewayKey, SharedSymmetricKey};
use sqlx::FromRow;
use time::OffsetDateTime;

pub struct Client {
    pub id: i64,
    pub client_type: crate::clients::ClientType,
}

#[derive(FromRow)]
pub struct PersistedSharedKeys {
    pub client_id: i64,

    #[allow(dead_code)]
    pub client_address_bs58: String,
    pub derived_aes128_ctr_blake3_hmac_keys_bs58: Option<String>,
    pub derived_aes256_gcm_siv_key: Option<Vec<u8>>,
    pub last_used_authentication: Option<OffsetDateTime>,
}

impl TryFrom<PersistedSharedKeys> for SharedGatewayKey {
    type Error = GatewayStorageError;

    fn try_from(value: PersistedSharedKeys) -> Result<Self, Self::Error> {
        match (
            &value.derived_aes256_gcm_siv_key,
            &value.derived_aes128_ctr_blake3_hmac_keys_bs58,
        ) {
            (None, None) => Err(GatewayStorageError::MissingSharedKey {
                id: value.client_id,
            }),
            (Some(aes256gcm_siv), _) => {
                let current_key = SharedSymmetricKey::try_from_bytes(aes256gcm_siv)
                    .map_err(|source| GatewayStorageError::DataCorruption(source.to_string()))?;
                Ok(SharedGatewayKey::Current(current_key))
            }
            (None, Some(aes128ctr_hmac)) => {
                let legacy_key = LegacySharedKeys::try_from_base58_string(aes128ctr_hmac)
                    .map_err(|source| GatewayStorageError::DataCorruption(source.to_string()))?;
                Ok(SharedGatewayKey::Legacy(legacy_key))
            }
        }
    }
}

#[derive(FromRow)]
pub struct StoredMessage {
    pub id: i64,
    #[allow(dead_code)]
    pub client_address_bs58: String,
    pub content: Vec<u8>,
    pub timestamp: OffsetDateTime,
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
    type Error = GatewayStorageError;

    fn try_from(value: UnverifiedTicketData) -> Result<Self, Self::Error> {
        Ok(ClientTicket {
            spending_data: CredentialSpendingData::try_from_bytes(&value.data).map_err(|_| {
                GatewayStorageError::MalformedStoredTicketData {
                    ticket_id: value.ticket_id,
                }
            })?,
            ticket_id: value.ticket_id,
        })
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct WireguardPeer {
    pub public_key: String,
    pub allowed_ips: Vec<u8>,
    pub client_id: i64,
}

impl WireguardPeer {
    pub fn from_defguard_peer(
        value: defguard_wireguard_rs::host::Peer,
        client_id: i64,
    ) -> Result<Self, crate::error::GatewayStorageError> {
        Ok(WireguardPeer {
            public_key: value.public_key.to_string(),
            allowed_ips: bincode::Options::serialize(
                bincode::DefaultOptions::new(),
                &value.allowed_ips,
            )
            .map_err(|e| {
                crate::error::GatewayStorageError::TypeConversion(format!("allowed ips {e}"))
            })?,
            client_id,
        })
    }
}

impl TryFrom<WireguardPeer> for defguard_wireguard_rs::host::Peer {
    type Error = crate::error::GatewayStorageError;

    fn try_from(value: WireguardPeer) -> Result<Self, Self::Error> {
        Ok(Self {
            public_key: value
                .public_key
                .as_str()
                .try_into()
                .map_err(|e| Self::Error::TypeConversion(format!("public key {e}")))?,
            allowed_ips: bincode::Options::deserialize(
                bincode::DefaultOptions::new(),
                &value.allowed_ips,
            )
            .map_err(|e| Self::Error::TypeConversion(format!("allowed ips {e}")))?,
            ..Default::default()
        })
    }
}
