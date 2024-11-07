// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Nym Statistics
//!
//! This crate contains basic statistics utilities and abstractions to be re-used and
//! applied throughout both the client and gateway implementations.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]
#![warn(missing_docs)]

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
    pub reporting_address: nym_sphinx::addressing::Recipient,

    /// Type of client reporting (vpn_client, authenticator, native-client)
    pub reporting_type: String,
}
/// vpn_client
pub const STATS_REPORTING_TYPE_VPN_CLIENT: &str = "vpn_client";

/// native_client
pub const STATS_REPORTING_TYPE_NATIVE_CLIENT: &str = "native_client";

/// unknown
pub const STATS_REPORTING_TYPE_UNKNOWN: &str = "unknown";
