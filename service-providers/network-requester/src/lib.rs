// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub mod config;
pub mod core;
pub mod error;
mod reply;
pub mod request_filter;
mod socks5;
mod statistics;

pub use crate::core::{NRServiceProvider, NRServiceProviderBuilder};
pub use config::Config;
pub use nym_client_core::{
    client::{
        base_client::{
            non_wasm_helpers::{setup_fs_gateways_storage, setup_fs_reply_surb_backend},
            storage::{
                helpers::{store_gateway_details, set_active_gateway}, GatewaysDetailsStore, OnDiskGatewaysDetails,
                OnDiskPersistent,gateways_storage::{GatewayRegistration, GatewayDetails, RemoteGatewayDetails, CustomGatewayDetails}
            },
        },
        key_manager::persistence::OnDiskKeys,
        mix_traffic::transceiver::*,
    },
    init::{
        generate_new_client_keys, setup_gateway,
        types::{GatewaySelectionSpecification, GatewaySetup, InitResults, InitialisationResult},
    },
};
pub use request_filter::RequestFilter;
