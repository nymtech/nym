// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::BadGateway;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::client::GatewayListeners;
use nym_gateway_requests::shared_key::SharedSymmetricKey;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use time::Duration;
use time::OffsetDateTime;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const REMOTE_GATEWAY_TYPE: &str = "remote";
pub const CUSTOM_GATEWAY_TYPE: &str = "custom";
const GATEWAY_DETAILS_TTL: Duration = Duration::days(7);

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
    pub fn gateway_id(&self) -> ed25519::PublicKey {
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
        gateway_id: ed25519::PublicKey,
        shared_key: Arc<SharedSymmetricKey>,
        published_data: GatewayPublishedData,
    ) -> Self {
        GatewayDetails::Remote(RemoteGatewayDetails {
            gateway_id,
            shared_key,
            published_data,
        })
    }

    pub fn new_custom(gateway_id: ed25519::PublicKey, data: Option<Vec<u8>>) -> Self {
        GatewayDetails::Custom(CustomGatewayDetails { gateway_id, data })
    }

    pub fn gateway_id(&self) -> ed25519::PublicKey {
        match self {
            GatewayDetails::Remote(details) => details.gateway_id,
            GatewayDetails::Custom(details) => details.gateway_id,
        }
    }

    pub fn shared_key(&self) -> Option<&SharedSymmetricKey> {
        match self {
            GatewayDetails::Remote(details) => Some(&details.shared_key),
            GatewayDetails::Custom(_) => None,
        }
    }

    pub fn details_exipration(&self) -> Option<OffsetDateTime> {
        match self {
            GatewayDetails::Remote(details) => Some(details.published_data.expiration_timestamp),
            GatewayDetails::Custom(_) => None,
        }
    }

    // pub fn update_remote_listeners(&mut self, new_listeners: GatewayListeners) {
    //     match self {
    //         GatewayDetails::Remote(details) => {
    //             details.gateway_listeners = new_listeners;
    //             details.expiration_timestamp = OffsetDateTime::now_utc() + GATEWAY_DETAILS_TTL;
    //         }
    //         GatewayDetails::Custom(_) => {}
    //     }
    // }

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
    pub gateway_id: ed25519::PublicKey,

    pub registration_timestamp: OffsetDateTime,

    pub gateway_type: GatewayType,
}

#[derive(Debug, Clone)]
pub struct GatewayPublishedData {
    pub listeners: GatewayListeners,
    pub expiration_timestamp: OffsetDateTime,
}

impl GatewayPublishedData {
    pub fn new(listeners: GatewayListeners) -> GatewayPublishedData {
        GatewayPublishedData {
            listeners,
            expiration_timestamp: OffsetDateTime::now_utc() + GATEWAY_DETAILS_TTL,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RawGatewayPublishedData {
    pub gateway_listener: String,
    pub fallback_listener: Option<String>,
    pub expiration_timestamp: OffsetDateTime,
}

impl<'a> From<&'a GatewayPublishedData> for RawGatewayPublishedData {
    fn from(value: &'a GatewayPublishedData) -> Self {
        Self {
            gateway_listener: value.listeners.primary.to_string(),
            fallback_listener: value.listeners.fallback.as_ref().map(|uri| uri.to_string()),
            expiration_timestamp: value.expiration_timestamp,
        }
    }
}

impl TryFrom<RawGatewayPublishedData> for GatewayPublishedData {
    type Error = BadGateway;

    fn try_from(value: RawGatewayPublishedData) -> Result<Self, Self::Error> {
        let gateway_listener: Url = Url::parse(&value.gateway_listener).map_err(|source| {
            BadGateway::MalformedListenerNoId {
                raw_listener: value.gateway_listener.clone(),
                source,
            }
        })?;
        let fallback_listener = value
            .fallback_listener
            .as_ref()
            .map(|uri| {
                Url::parse(uri).map_err(|source| BadGateway::MalformedListenerNoId {
                    raw_listener: uri.to_owned(),
                    source,
                })
            })
            .transpose()?;

        Ok(GatewayPublishedData {
            listeners: GatewayListeners {
                primary: gateway_listener,
                fallback: fallback_listener,
            },
            expiration_timestamp: value.expiration_timestamp,
        })
    }
}

#[derive(Debug, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RawRemoteGatewayDetails {
    pub gateway_id_bs58: String,
    pub derived_aes256_gcm_siv_key: Vec<u8>,
    #[zeroize(skip)]
    #[cfg_attr(feature = "sqlx", sqlx(flatten))]
    pub published_data: RawGatewayPublishedData,
}

impl TryFrom<RawRemoteGatewayDetails> for RemoteGatewayDetails {
    type Error = BadGateway;

    fn try_from(value: RawRemoteGatewayDetails) -> Result<Self, Self::Error> {
        let gateway_id =
            ed25519::PublicKey::from_base58_string(&value.gateway_id_bs58).map_err(|source| {
                BadGateway::MalformedGatewayIdentity {
                    gateway_id: value.gateway_id_bs58.clone(),
                    source,
                }
            })?;

        let shared_key = SharedSymmetricKey::try_from_bytes(&value.derived_aes256_gcm_siv_key)
            .map_err(|source| BadGateway::MalformedSharedKeys {
                gateway_id: value.gateway_id_bs58.clone(),
                source,
            })?;

        Ok(RemoteGatewayDetails {
            gateway_id,
            shared_key: Arc::new(shared_key),
            published_data: value.published_data.clone().try_into()?,
        })
    }
}

impl<'a> From<&'a RemoteGatewayDetails> for RawRemoteGatewayDetails {
    fn from(value: &'a RemoteGatewayDetails) -> Self {
        RawRemoteGatewayDetails {
            gateway_id_bs58: value.gateway_id.to_base58_string(),
            derived_aes256_gcm_siv_key: value.shared_key.to_bytes(),
            published_data: (&value.published_data).into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteGatewayDetails {
    pub gateway_id: ed25519::PublicKey,

    pub shared_key: Arc<SharedSymmetricKey>,

    pub published_data: GatewayPublishedData,
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
            ed25519::PublicKey::from_base58_string(&value.gateway_id_bs58).map_err(|source| {
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
    pub gateway_id: ed25519::PublicKey,
    pub data: Option<Vec<u8>>,
}

impl CustomGatewayDetails {
    pub fn new(gateway_id: ed25519::PublicKey) -> CustomGatewayDetails {
        Self {
            gateway_id,
            data: None,
        }
    }
}
