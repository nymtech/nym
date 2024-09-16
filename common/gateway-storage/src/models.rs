// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::StorageError;
use nym_credentials_interface::{AvailableBandwidth, ClientTicket, CredentialSpendingData};
use nym_gateway_requests::registration::handshake::{
    LegacySharedKeys, SharedGatewayKey, SharedSymmetricKey,
};
use sqlx::FromRow;
use time::OffsetDateTime;

pub struct Client {
    pub id: i64,
    pub client_type: crate::clients::ClientType,
}

#[derive(FromRow)]
pub struct PersistedSharedKeys {
    #[allow(dead_code)]
    pub client_id: i64,

    #[allow(dead_code)]
    pub client_address_bs58: String,
    pub derived_aes128_ctr_blake3_hmac_keys_bs58: Option<String>,
    pub derived_aes256_gcm_siv_key: Option<Vec<u8>>,
}

impl TryFrom<PersistedSharedKeys> for SharedGatewayKey {
    type Error = StorageError;

    fn try_from(value: PersistedSharedKeys) -> Result<Self, Self::Error> {
        match (
            &value.derived_aes256_gcm_siv_key,
            &value.derived_aes128_ctr_blake3_hmac_keys_bs58,
        ) {
            (None, None) => Err(StorageError::MissingSharedKey { id: value.client_id }),
            (Some(aes256gcm_siv), _) => {
                let current_key = SharedSymmetricKey::try_from_bytes(aes256gcm_siv)
                    .map_err(|source| StorageError::DataCorruption(source.to_string()))?;
                Ok(SharedGatewayKey::Current(current_key))
            }
            (None, Some(aes128ctr_hmac)) => {
                let legacy_key = LegacySharedKeys::try_from_base58_string(aes128ctr_hmac)
                    .map_err(|source| StorageError::DataCorruption(source.to_string()))?;
                Ok(SharedGatewayKey::Legacy(legacy_key))
            }
        }
    }
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

impl From<defguard_wireguard_rs::host::Peer> for WireguardPeer {
    fn from(value: defguard_wireguard_rs::host::Peer) -> Self {
        WireguardPeer {
            public_key: value.public_key.to_string(),
            preshared_key: value.preshared_key.as_ref().map(|k| k.to_string()),
            protocol_version: value.protocol_version.map(|v| v as i64),
            endpoint: value.endpoint.map(|e| e.to_string()),
            last_handshake: value.last_handshake.and_then(|t| {
                if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
                    if let Ok(millis) = d.as_millis().try_into() {
                        sqlx::types::chrono::DateTime::from_timestamp_millis(millis)
                            .map(|d| d.naive_utc())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }),
            tx_bytes: value.tx_bytes as i64,
            rx_bytes: value.rx_bytes as i64,
            persistent_keepalive_interval: value.persistent_keepalive_interval.map(|v| v as i64),
            allowed_ips: bincode::Options::serialize(
                bincode::DefaultOptions::new(),
                &value.allowed_ips,
            )
            .unwrap_or_default(),
            suspended: false,
        }
    }
}

impl TryFrom<WireguardPeer> for defguard_wireguard_rs::host::Peer {
    type Error = crate::error::StorageError;

    fn try_from(value: WireguardPeer) -> Result<Self, Self::Error> {
        Ok(Self {
            public_key: value
                .public_key
                .as_str()
                .try_into()
                .map_err(|e| Self::Error::TypeConversion(format!("public key {e}")))?,
            preshared_key: value
                .preshared_key
                .as_deref()
                .map(TryFrom::try_from)
                .transpose()
                .map_err(|e| Self::Error::TypeConversion(format!("preshared key {e}")))?,
            protocol_version: value
                .protocol_version
                .map(TryFrom::try_from)
                .transpose()
                .map_err(|e| Self::Error::TypeConversion(format!("protocol version {e}")))?,
            endpoint: value
                .endpoint
                .as_deref()
                .map(|e| e.parse())
                .transpose()
                .map_err(|e| Self::Error::TypeConversion(format!("endpoint {e}")))?,
            last_handshake: value.last_handshake.and_then(|t| {
                let unix_time = std::time::UNIX_EPOCH;
                if let Ok(millis) = t.and_utc().timestamp_millis().try_into() {
                    let duration = std::time::Duration::from_millis(millis);
                    unix_time.checked_add(duration)
                } else {
                    None
                }
            }),
            tx_bytes: value
                .tx_bytes
                .try_into()
                .map_err(|e| Self::Error::TypeConversion(format!("tx bytes {e}")))?,
            rx_bytes: value
                .rx_bytes
                .try_into()
                .map_err(|e| Self::Error::TypeConversion(format!("rx bytes {e}")))?,
            persistent_keepalive_interval: value
                .persistent_keepalive_interval
                .map(TryFrom::try_from)
                .transpose()
                .map_err(|e| {
                    Self::Error::TypeConversion(format!("persistent keepalive interval {e}"))
                })?,
            allowed_ips: bincode::Options::deserialize(
                bincode::DefaultOptions::new(),
                &value.allowed_ips,
            )
            .map_err(|e| Self::Error::TypeConversion(format!("allowed ips {e}")))?,
        })
    }
}
