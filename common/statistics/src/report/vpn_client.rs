// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

const KIND: &str = "vpn_client_stats_report";
const VERSION: &str = "v1";

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnClientStatsReport {
    pub kind: String,
    pub api_version: String,
    pub stats_id: String,
    pub static_information: StaticInformationReport,
    //SW called it basic so we can swap it easily down the line for more data
    pub basic_usage: Option<UsageReport>,
}

impl VpnClientStatsReport {
    pub fn new(stats_id: String, static_information: StaticInformationReport) -> Self {
        VpnClientStatsReport {
            kind: KIND.into(),
            api_version: VERSION.into(),
            stats_id,
            static_information,
            basic_usage: None,
        }
    }

    #[must_use]
    pub fn with_usage_report(mut self, usage_report: UsageReport) -> Self {
        self.basic_usage = Some(usage_report);
        self
    }
}
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticInformationReport {
    pub os_type: String,
    pub os_version: Option<String>,
    pub os_arch: String,
    pub app_version: String,
}

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub connection_time_ms: Option<i32>,
    pub two_hop: bool,
}
