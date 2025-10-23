// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_contracts_common::Percent;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn split_ips(ips: Vec<IpAddr>) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    ips.into_iter()
        .fold((vec![], vec![]), |(mut v4, mut v6), ip| {
            match ip {
                IpAddr::V4(ipv4_addr) => v4.push(ipv4_addr),
                IpAddr::V6(ipv6_addr) => v6.push(ipv6_addr),
            }
            (v4, v6)
        })
}

// Types copied in from nym-vpn-client/nym-vpn-core/crates/nym-vpn-api-client

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NymDirectoryCountry(String);

impl NymDirectoryCountry {
    pub fn iso_code(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for NymDirectoryCountry {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreThresholds {
    pub high: u8,
    pub medium: u8,
    pub low: u8,
}

#[derive(Clone, Debug)]
pub enum GatewayType {
    MixnetEntry,
    MixnetExit,
    Wg,
}

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct GatewayMinPerformance {
    pub mixnet_min_performance: Option<Percent>,
    pub vpn_min_performance: Option<Percent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BridgeInformation {
    pub version: String,
    pub transports: Vec<BridgeParameters>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "transport_type", content = "args")]
#[serde(rename_all = "snake_case")]
pub enum BridgeParameters {
    QuicPlain(QuicClientOptions),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicClientOptions {
    /// Address describing the remote transport server. This is a vec to support multiple addresses
    /// so as to support both IPv4 and IPv6. These addresses are meant to describe a single bridge
    /// as the key material should not be used across multiple instances.
    pub addresses: Vec<std::net::SocketAddr>,

    /// Override hostname used for certificate verification
    pub host: Option<String>,

    /// Use identity public key to verify server self signed certificate
    pub id_pubkey: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AsnKind {
    Residential,
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Asn {
    pub asn: String,
    pub name: String,
    pub kind: AsnKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Location {
    pub two_letter_iso_country_code: String,
    pub latitude: f64,
    pub longitude: f64,

    pub city: String,
    pub region: String,

    pub asn: Option<Asn>,
}
