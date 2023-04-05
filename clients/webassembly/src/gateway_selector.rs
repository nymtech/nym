// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::GatewayEndpointConfig;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn get_gateway(api_server: String, preferred: Option<String>) -> GatewayEndpointConfig {
    let validator_client = nym_validator_client::client::NymApiClient::new(api_server.parse().unwrap());

    let gateways = match validator_client.get_cached_gateways().await {
        Err(err) => panic!("failed to obtain list of all gateways - {err}"),
        Ok(gateways) => gateways,
    };

    if let Some(preferred) = preferred {
        if let Some(details) = gateways
            .iter()
            .find(|g| g.gateway.identity_key == preferred)
        {
            return GatewayEndpointConfig {
                gateway_id: details.gateway.identity_key.clone(),
                gateway_owner: details.owner.to_string(),
                gateway_listener: format!(
                    "ws://{}:{}",
                    details.gateway.host, details.gateway.clients_port
                ),
            };
        }
    }

    let details = gateways
        .first()
        .expect("current topology holds no gateways");

    GatewayEndpointConfig {
        gateway_id: details.gateway.identity_key.clone(),
        gateway_owner: details.owner.to_string(),
        gateway_listener: format!(
            "ws://{}:{}",
            details.gateway.host, details.gateway.clients_port
        ),
    }
}
