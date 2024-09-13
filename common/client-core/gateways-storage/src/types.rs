// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::BadGateway;
use cosmrs::AccountId;
use nym_crypto::asymmetric::identity;
use nym_gateway_requests::registration::handshake::LegacySharedKeys;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use time::OffsetDateTime;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const REMOTE_GATEWAY_TYPE: &str = "remote";
pub const CUSTOM_GATEWAY_TYPE: &str = "custom";

#[derive(Debug, Clone, Default)]
pub struct ActiveGateway {
    pub registration: Option<GatewayRegistration>,
}

#[derive(Debug, Clone)]
pub struct GatewayRegistration {
    pub details: GatewayDetails,
    pub registration_timestamp: OffsetDateTime,
}

impl GatewayRegistration {
    pub fn gateway_id(&self) -> identity::PublicKey {
        self.details.gateway_id()
    }
}

impl<'a> From<&'a GatewayRegistration> for RawRegisteredGateway {
    fn from(value: &'a GatewayRegistration) -> Self {
        RawRegisteredGateway {
            gateway_id_bs58: value.details.gateway_id().to_base58_string(),
            registration_timestamp: value.registration_timestamp,
            gateway_type: value.details.typ().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GatewayDetails {
    /// Standard details of a remote gateway
    Remote(RemoteGatewayDetails),

    /// Custom gateway setup, such as for a client embedded inside gateway itself
    Custom(CustomGatewayDetails),
}

impl From<GatewayDetails> for GatewayRegistration {
    fn from(details: GatewayDetails) -> Self {
        GatewayRegistration {
            details,
            registration_timestamp: OffsetDateTime::now_utc(),
        }
    }
}

impl GatewayDetails {
    pub fn new_remote(
        gateway_id: identity::PublicKey,
        derived_aes128_ctr_blake3_hmac_keys: Arc<LegacySharedKeys>,
        gateway_owner_address: Option<AccountId>,
        gateway_listener: Url,
    ) -> Self {
        GatewayDetails::Remote(RemoteGatewayDetails {
            gateway_id,
            derived_aes128_ctr_blake3_hmac_keys,
            gateway_owner_address,
            gateway_listener,
        })
    }

    pub fn new_custom(gateway_id: identity::PublicKey, data: Option<Vec<u8>>) -> Self {
        GatewayDetails::Custom(CustomGatewayDetails { gateway_id, data })
    }

    pub fn gateway_id(&self) -> identity::PublicKey {
        match self {
            GatewayDetails::Remote(details) => details.gateway_id,
            GatewayDetails::Custom(details) => details.gateway_id,
        }
    }

    pub fn shared_key(&self) -> Option<&LegacySharedKeys> {
        match self {
            GatewayDetails::Remote(details) => Some(&details.derived_aes128_ctr_blake3_hmac_keys),
            GatewayDetails::Custom(_) => None,
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, GatewayDetails::Custom(..))
    }

    pub fn typ(&self) -> GatewayType {
        match self {
            GatewayDetails::Remote(_) => GatewayType::Remote,
            GatewayDetails::Custom(_) => GatewayType::Custom,
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub enum GatewayType {
    #[default]
    Remote,

    Custom,
}

impl FromStr for GatewayType {
    type Err = BadGateway;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            REMOTE_GATEWAY_TYPE => Ok(GatewayType::Remote),
            CUSTOM_GATEWAY_TYPE => Ok(GatewayType::Custom),
            other => Err(BadGateway::InvalidGatewayType {
                typ: other.to_string(),
            }),
        }
    }
}

impl Display for GatewayType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayType::Remote => REMOTE_GATEWAY_TYPE.fmt(f),
            GatewayType::Custom => CUSTOM_GATEWAY_TYPE.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RawActiveGateway {
    pub active_gateway_id_bs58: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RawRegisteredGateway {
    pub gateway_id_bs58: String,

    // not necessarily needed but is nice for display purposes
    pub registration_timestamp: OffsetDateTime,

    pub gateway_type: String,
}

#[derive(Debug, Clone, Copy)]
pub struct RegisteredGateway {
    pub gateway_id: identity::PublicKey,

    pub registration_timestamp: OffsetDateTime,

    pub gateway_type: GatewayType,
}

#[derive(Debug, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RawRemoteGatewayDetails {
    pub gateway_id_bs58: String,
    pub derived_aes128_ctr_blake3_hmac_keys_bs58: String,
    pub gateway_owner_address: Option<String>,
    pub gateway_listener: String,
}

impl TryFrom<RawRemoteGatewayDetails> for RemoteGatewayDetails {
    type Error = BadGateway;

    fn try_from(value: RawRemoteGatewayDetails) -> Result<Self, Self::Error> {
        let gateway_id =
            identity::PublicKey::from_base58_string(&value.gateway_id_bs58).map_err(|source| {
                BadGateway::MalformedGatewayIdentity {
                    gateway_id: value.gateway_id_bs58.clone(),
                    source,
                }
            })?;

        let derived_aes128_ctr_blake3_hmac_keys = Arc::new(
            LegacySharedKeys::try_from_base58_string(
                &value.derived_aes128_ctr_blake3_hmac_keys_bs58,
            )
            .map_err(|source| BadGateway::MalformedSharedKeys {
                gateway_id: value.gateway_id_bs58.clone(),
                source,
            })?,
        );

        let gateway_owner_address = value
            .gateway_owner_address
            .as_ref()
            .map(|raw_owner| {
                AccountId::from_str(raw_owner).map_err(|source| {
                    BadGateway::MalformedGatewayOwnerAccountAddress {
                        gateway_id: value.gateway_id_bs58.clone(),
                        raw_owner: raw_owner.clone(),
                        source,
                    }
                })
            })
            .transpose()?;

        let gateway_listener = Url::parse(&value.gateway_listener).map_err(|source| {
            BadGateway::MalformedListener {
                gateway_id: value.gateway_id_bs58.clone(),
                raw_listener: value.gateway_listener.clone(),
                source,
            }
        })?;

        Ok(RemoteGatewayDetails {
            gateway_id,
            derived_aes128_ctr_blake3_hmac_keys,
            gateway_owner_address,
            gateway_listener,
        })
    }
}

impl<'a> From<&'a RemoteGatewayDetails> for RawRemoteGatewayDetails {
    fn from(value: &'a RemoteGatewayDetails) -> Self {
        RawRemoteGatewayDetails {
            gateway_id_bs58: value.gateway_id.to_base58_string(),
            derived_aes128_ctr_blake3_hmac_keys_bs58: value
                .derived_aes128_ctr_blake3_hmac_keys
                .to_base58_string(),
            gateway_owner_address: value.gateway_owner_address.as_ref().map(|o| o.to_string()),
            gateway_listener: value.gateway_listener.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteGatewayDetails {
    pub gateway_id: identity::PublicKey,

    // note: `SharedKeys` implement ZeroizeOnDrop, meaning when `RemoteGatewayDetails` is dropped,
    // the keys will be zeroized
    pub derived_aes128_ctr_blake3_hmac_keys: Arc<LegacySharedKeys>,

    pub gateway_owner_address: Option<AccountId>,

    pub gateway_listener: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RawCustomGatewayDetails {
    pub gateway_id_bs58: String,
    pub data: Option<Vec<u8>>,
}

impl TryFrom<RawCustomGatewayDetails> for CustomGatewayDetails {
    type Error = BadGateway;

    fn try_from(value: RawCustomGatewayDetails) -> Result<Self, Self::Error> {
        let gateway_id =
            identity::PublicKey::from_base58_string(&value.gateway_id_bs58).map_err(|source| {
                BadGateway::MalformedGatewayIdentity {
                    gateway_id: value.gateway_id_bs58.clone(),
                    source,
                }
            })?;

        Ok(CustomGatewayDetails {
            gateway_id,
            data: value.data,
        })
    }
}

impl<'a> From<&'a CustomGatewayDetails> for RawCustomGatewayDetails {
    fn from(value: &'a CustomGatewayDetails) -> Self {
        RawCustomGatewayDetails {
            gateway_id_bs58: value.gateway_id.to_base58_string(),
            // I don't know what to feel about that clone here given it might contain possibly sensitive data
            data: value.data.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CustomGatewayDetails {
    pub gateway_id: identity::PublicKey,
    pub data: Option<Vec<u8>>,
}

impl CustomGatewayDetails {
    pub fn new(gateway_id: identity::PublicKey) -> CustomGatewayDetails {
        Self {
            gateway_id,
            data: None,
        }
    }
}
