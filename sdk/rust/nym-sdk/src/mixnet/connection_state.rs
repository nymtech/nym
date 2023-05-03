// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::client::key_manager::{KeyManager, KeyManagerBuilder};
use nym_client_core::config::GatewayEndpointConfig;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum BuilderState {
    New {
        initial_keys: KeyManagerBuilder,
    },
    Registered {
        derived_keys: KeyManager,
        gateway_endpoint_config: GatewayEndpointConfig,
    },
}

impl BuilderState {
    pub(super) fn gateway_endpoint_config(&self) -> Option<&GatewayEndpointConfig> {
        match self {
            BuilderState::New => None,
            BuilderState::Registered {
                gateway_endpoint_config,
                ..
            } => Some(gateway_endpoint_config),
        }
    }
}
