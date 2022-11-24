// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use client_core::config::GatewayEndpoint;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn get_gateway(api_server: String, preferred: Option<String>) -> GatewayEndpoint {
    let validator_client = validator_client::client::ApiClient::new(api_server.parse().unwrap());

    let gateways = match validator_client.get_cached_gateways().await {
        Err(err) => panic!("failed to obtain list of all gateways - {}", err),
        Ok(gateways) => gateways,
    };

    if let Some(preferred) = preferred {
        if let Some(details) = gateways
            .iter()
            .find(|g| g.gateway.identity_key == preferred)
        {
            return GatewayEndpoint {
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

    GatewayEndpoint {
        gateway_id: details.gateway.identity_key.clone(),
        gateway_owner: details.owner.to_string(),
        gateway_listener: format!(
            "ws://{}:{}",
            details.gateway.host, details.gateway.clients_port
        ),
    }
}
