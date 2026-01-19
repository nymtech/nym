// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::client::base_client::storage::gateways_storage::{
    BadGateway, GatewayDetails, GatewayPublishedData, GatewayRegistration, RawGatewayPublishedData,
    RawRemoteGatewayDetails, RemoteGatewayDetails,
};
use nym_topology::EntryDetails;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use zeroize::Zeroize;

// a more nested struct since we only have a single gateway type in wasm (no 'custom')
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
pub struct WasmRawRegisteredGateway {
    pub gateway_id_bs58: String,

    #[zeroize(skip)]
    pub registration_timestamp: OffsetDateTime,

    #[serde(default)]
    pub derived_aes256_gcm_siv_key: Vec<u8>,

    #[zeroize(skip)]
    pub published_data: WasmRawGatewayPublishedData,
}

impl TryFrom<WasmRawRegisteredGateway> for GatewayRegistration {
    type Error = BadGateway;

    fn try_from(value: WasmRawRegisteredGateway) -> Result<Self, Self::Error> {
        // offload some parsing to an existing impl
        let raw_remote = RawRemoteGatewayDetails {
            gateway_id_bs58: value.gateway_id_bs58,
            derived_aes256_gcm_siv_key: value.derived_aes256_gcm_siv_key,
            published_data: RawGatewayPublishedData {
                gateway_details: value.published_data.gateway_details,
                expiration_timestamp: value.published_data.expiration_timestamp,
            },
        };
        let remote: RemoteGatewayDetails = raw_remote.try_into()?;

        Ok(GatewayRegistration {
            details: GatewayDetails::Remote(remote),
            registration_timestamp: value.registration_timestamp,
        })
    }
}

impl<'a> From<&'a GatewayRegistration> for WasmRawRegisteredGateway {
    fn from(value: &'a GatewayRegistration) -> Self {
        let GatewayDetails::Remote(remote_details) = &value.details else {
            panic!("somehow obtained custom gateway registration in wasm!")
        };

        let derived_aes256_gcm_siv_key = remote_details.shared_key.to_bytes().to_vec();

        WasmRawRegisteredGateway {
            gateway_id_bs58: remote_details.gateway_id.to_string(),
            registration_timestamp: value.registration_timestamp,
            derived_aes256_gcm_siv_key,
            published_data: (&remote_details.published_data).into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRawGatewayPublishedData {
    pub gateway_details: EntryDetails,

    pub expiration_timestamp: OffsetDateTime,
}

impl<'a> From<&'a GatewayPublishedData> for WasmRawGatewayPublishedData {
    fn from(value: &'a GatewayPublishedData) -> Self {
        WasmRawGatewayPublishedData {
            gateway_details: value.details.clone(),
            expiration_timestamp: value.expiration_timestamp,
        }
    }
}
