// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Nym Statistics
//!
//! This crate contains basic statistics utilities and abstractions to be re-used and
//! applied throughout both the client and gateway implementations.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use nym_sphinx::addressing::Recipient;

/// Client specific statistics interfaces and events.
pub mod clients;
/// Statistics related errors.
pub mod error;
/// Gateway specific statistics interfaces and events.
pub mod gateways;
/// Statistics reporting abstractions and implementations.
pub mod report;

/// Stats Reporting config to use when stats reporting is wanted
#[derive(Clone, Debug)]
pub struct StatsReportingConfig {
    /// Client address to report to
    pub reporting_address: Recipient,

    /// Type of client reporting (vpn_client, authenticator, native-client)
    pub reporting_type: String,
}

impl StatsReportingConfig {
    /// Create a StatsReportingConfig for a native client
    pub fn new_native(reporting_address: Recipient) -> Self {
        StatsReportingConfig {
            reporting_address,
            reporting_type: NATIVE_CLIENT.to_string(),
        }
    }
    /// Create a StatsReportingConfig for a vpn client
    pub fn new_vpn(reporting_address: Recipient) -> Self {
        StatsReportingConfig {
            reporting_address,
            reporting_type: VPN_CLIENT.to_string(),
        }
    }
    /// Create a StatsReportingConfig for a socks5 client
    pub fn new_socks5(reporting_address: Recipient) -> Self {
        StatsReportingConfig {
            reporting_address,
            reporting_type: SOCKS5_CLIENT.to_string(),
        }
    }
    /// Create a StatsReportingConfig for an unspecified client
    pub fn new_unknown(reporting_address: Recipient) -> Self {
        StatsReportingConfig {
            reporting_address,
            reporting_type: UNKNOWN.to_string(),
        }
    }
}

const VPN_CLIENT: &str = "vpn_client";
const NATIVE_CLIENT: &str = "native_client";
const SOCKS5_CLIENT: &str = "socks5_client";
const UNKNOWN: &str = "unknown";
