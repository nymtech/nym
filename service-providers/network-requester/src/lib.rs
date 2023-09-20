// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod allowed_hosts;
pub mod config;
pub mod core;
pub mod error;
mod reply;
mod socks5;
mod statistics;

pub use crate::core::{NRServiceProvider, NRServiceProviderBuilder};
pub use config::Config;
pub use nym_client_core::{
    client::{
        base_client::storage::{gateway_details::OnDiskGatewayDetails, OnDiskPersistent},
        key_manager::persistence::OnDiskKeys,
        mix_traffic::transceiver::*,
    },
    init::{
        setup_gateway,
        types::{
            CustomGatewayDetails, GatewayDetails, GatewaySelectionSpecification, GatewaySetup,
            InitResults, InitialisationResult,
        },
    },
};
