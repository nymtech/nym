// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::client::base_client::storage::gateways_storage::{
    BadGateway, GatewayDetails, GatewayRegistration, RawRemoteGatewayDetails, RemoteGatewayDetails,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// a more nested struct since we only have a single gateway type in wasm (no 'custom')
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRawRegisteredGateway {
    pub gateway_id_bs58: String,

    pub registration_timestamp: OffsetDateTime,

    pub derived_aes128_ctr_blake3_hmac_keys_bs58: String,

    pub gateway_owner_address: String,

    pub gateway_listener: String,
}

impl TryFrom<WasmRawRegisteredGateway> for GatewayRegistration {
    type Error = BadGateway;

    fn try_from(value: WasmRawRegisteredGateway) -> Result<Self, Self::Error> {
        // offload some parsing to an existing impl
        let raw_remote = RawRemoteGatewayDetails {
            gateway_id_bs58: value.gateway_id_bs58,
            derived_aes128_ctr_blake3_hmac_keys_bs58: value
                .derived_aes128_ctr_blake3_hmac_keys_bs58,
            gateway_owner_address: value.gateway_owner_address,
            gateway_listener: value.gateway_listener,
            wg_tun_address: None,
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

        WasmRawRegisteredGateway {
            gateway_id_bs58: remote_details.gateway_id.to_string(),
            registration_timestamp: value.registration_timestamp,
            derived_aes128_ctr_blake3_hmac_keys_bs58: remote_details
                .derived_aes128_ctr_blake3_hmac_keys
                .to_base58_string(),
            gateway_owner_address: remote_details.gateway_owner_address.to_string(),
            gateway_listener: remote_details.gateway_listener.to_string(),
        }
    }
}
