// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod entries;
mod error;
mod gateway_client;
mod helpers;
pub use nym_vpn_api_client::types::{GatewayMinPerformance, NaiveFloat, Percent};

pub use crate::{
    entries::{
        auth_addresses::AuthAddress,
        country::Country,
        entry_point::EntryPoint,
        exit_point::ExitPoint,
        gateway::{
            Asn, AsnKind, Entry, Exit, Gateway, GatewayFilter, GatewayList, GatewayType, Location,
            Performance, Probe, ProbeOutcome, ScoreValue,
        },
        ipr_addresses::IpPacketRouterAddress,
        score::Score,
    },
    error::Error,
    gateway_client::{Config, GatewayClient},
    helpers::split_ips,
};
