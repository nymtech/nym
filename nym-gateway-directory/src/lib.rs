// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod caching_client;
mod entries;
mod error;
mod gateway_client;
mod helpers;

pub use nym_sdk::mixnet::{NodeIdentity, Recipient};
pub use nym_vpn_api_client::types::{GatewayMinPerformance, Percent};

pub use crate::{
    caching_client::CachingGatewayClient,
    entries::{
        auth_addresses::{AuthAddress, AuthAddresses},
        country::Country,
        entry_point::EntryPoint,
        exit_point::ExitPoint,
        gateway::{
            Entry, Exit, Gateway, GatewayList, GatewayType, Location, NymNode, Probe, ProbeOutcome,
        },
        ipr_addresses::IpPacketRouterAddress,
        score::Score,
    },
    error::Error,
    gateway_client::{Config, GatewayClient, ResolvedConfig},
    helpers::resolve_config,
};
