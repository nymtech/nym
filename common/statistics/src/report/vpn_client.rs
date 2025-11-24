// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use time::Date;

const BASIC_REPORT_KIND: &str = "vpn_client_stats_report";
const SESSION_REPORT_KIND: &str = "vpn_client_session_report";
const VERSION_1: &str = "v1";
const VERSION_2: &str = "v2";

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnClientStatsReport {
    pub kind: String,
    pub api_version: String,
    pub stats_id: String,
    pub static_information: StaticInformationReport,
    pub basic_usage: Option<UsageReport>,
}

impl VpnClientStatsReport {
    pub fn new(stats_id: String, static_information: StaticInformationReport) -> Self {
        VpnClientStatsReport {
            kind: BASIC_REPORT_KIND.into(),
            api_version: VERSION_1.into(),
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
pub struct VpnClientStatsReportV2 {
    pub kind: String,
    pub api_version: String,
    pub stats_id: String,

    pub static_information: StaticInformationReport,
    pub session_report: SessionReport,
}

impl VpnClientStatsReportV2 {
    pub fn new(
        stats_id: String,
        static_information: StaticInformationReport,
        session_report: SessionReport,
    ) -> Self {
        VpnClientStatsReportV2 {
            kind: SESSION_REPORT_KIND.into(),
            api_version: VERSION_2.into(),
            stats_id,
            static_information,
            session_report,
        }
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

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReport {
    pub start_day: Date,
    pub connection_time_ms: i32,
    pub tunnel_type: String,
    pub retry_attempt: i32,
    pub session_duration_min: i32,
    pub disconnection_time_ms: i32,
    pub exit_id: String,
    pub follow_up_id: Option<String>,
    pub error: Option<String>,
}
