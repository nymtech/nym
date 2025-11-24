use std::net::IpAddr;

use axum_extra::headers::UserAgent;
use celes::Country;
use nym_statistics_common::report::vpn_client::{VpnClientStatsReport, VpnClientStatsReportV2};
use time::{Date, OffsetDateTime};

pub type StatsId = String;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct DailyActiveDeviceDto {
    pub(crate) day: Date,
    pub(crate) stats_id: StatsId,
    pub(crate) os_type: String,
    pub(crate) os_version: Option<String>,
    pub(crate) os_arch: String,
    pub(crate) app_version: String,
    pub(crate) user_agent: String,
    pub(crate) from_mixnet: bool,
}

impl DailyActiveDeviceDto {
    pub(crate) fn new(
        received_at: OffsetDateTime,
        stats_report: &VpnClientStatsReport,
        user_agent: UserAgent,
        from_mixnet: bool,
    ) -> Self {
        Self {
            day: received_at.date(),
            stats_id: stats_report.stats_id.clone(),
            os_type: stats_report.static_information.os_type.clone(),
            os_version: stats_report.static_information.os_version.clone(),
            os_arch: stats_report.static_information.os_arch.clone(),
            app_version: stats_report.static_information.app_version.clone(),
            user_agent: user_agent.to_string(),
            from_mixnet,
        }
    }
    pub(crate) fn new_v2(
        received_at: OffsetDateTime,
        stats_report: &VpnClientStatsReportV2,
        user_agent: UserAgent,
        from_mixnet: bool,
    ) -> Self {
        Self {
            day: received_at.date(),
            stats_id: stats_report.stats_id.clone(),
            os_type: stats_report.static_information.os_type.clone(),
            os_version: stats_report.static_information.os_version.clone(),
            os_arch: stats_report.static_information.os_arch.clone(),
            app_version: stats_report.static_information.app_version.clone(),
            user_agent: user_agent.to_string(),
            from_mixnet,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct StatsReportV1Dto {
    pub(crate) received_at: OffsetDateTime,
    pub(crate) received_from: String,
    pub(crate) stats_id: StatsId,
    pub(crate) from_mixnet: bool,
    pub(crate) os_type: String,
    pub(crate) os_version: Option<String>,
    pub(crate) os_arch: String,
    pub(crate) app_version: String,
    pub(crate) user_agent: String,
    pub(crate) connection_time_ms: Option<i32>,
    pub(crate) two_hop: Option<bool>,
    pub(crate) country_code: Option<String>,
}

impl StatsReportV1Dto {
    pub(crate) fn new(
        received_at: OffsetDateTime,
        stats_report: &VpnClientStatsReport,
        user_agent: UserAgent,
        from_mixnet: bool,
        received_from: IpAddr,
        maybe_country: Option<Country>,
    ) -> Self {
        let mut report = Self {
            received_at,
            received_from: received_from.to_string(),
            stats_id: stats_report.stats_id.clone(),
            from_mixnet,
            os_type: stats_report.static_information.os_type.clone(),
            os_version: stats_report.static_information.os_version.clone(),
            os_arch: stats_report.static_information.os_arch.clone(),
            app_version: stats_report.static_information.app_version.clone(),
            user_agent: user_agent.to_string(),
            connection_time_ms: None,
            two_hop: None,
            country_code: maybe_country.map(|c| c.alpha2.into()),
        };
        if let Some(usage_report) = stats_report.basic_usage.as_ref() {
            report.connection_time_ms = usage_report.connection_time_ms;
            report.two_hop = Some(usage_report.two_hop);
        }

        report
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct StatsReportV2Dto {
    // Report metadata
    pub(crate) received_at: OffsetDateTime,
    pub(crate) received_from: String,
    pub(crate) from_mixnet: bool,
    pub(crate) country_code: Option<String>,
    pub(crate) report_version: String,

    // Device info
    pub(crate) stats_id: StatsId,
    pub(crate) os_type: String,
    pub(crate) os_version: Option<String>,
    pub(crate) os_arch: String,
    pub(crate) app_version: String,
    pub(crate) user_agent: String,

    // session info
    pub(crate) start_day: Date,
    pub(crate) connection_time_ms: i32,
    pub(crate) tunnel_type: String,
    pub(crate) retry_attempt: i32,
    pub(crate) session_duration_min: i32,
    pub(crate) disconnection_time_ms: i32,
    pub(crate) exit_id: String,
    pub(crate) follow_up_id: Option<String>,
    pub(crate) error: Option<String>,
}

impl StatsReportV2Dto {
    pub(crate) fn new(
        received_at: OffsetDateTime,
        stats_report: &VpnClientStatsReportV2,
        user_agent: UserAgent,
        from_mixnet: bool,
        received_from: IpAddr,
        maybe_country: Option<Country>,
    ) -> Self {
        Self {
            received_at,
            received_from: received_from.to_string(),
            from_mixnet,
            country_code: maybe_country.map(|c| c.alpha2.into()),
            report_version: stats_report.api_version.clone(),
            stats_id: stats_report.stats_id.clone(),
            os_type: stats_report.static_information.os_type.clone(),
            os_version: stats_report.static_information.os_version.clone(),
            os_arch: stats_report.static_information.os_arch.clone(),
            app_version: stats_report.static_information.app_version.clone(),
            user_agent: user_agent.to_string(),
            start_day: stats_report.session_report.start_day,
            connection_time_ms: stats_report.session_report.connection_time_ms,
            tunnel_type: stats_report.session_report.tunnel_type.clone(),
            retry_attempt: stats_report.session_report.retry_attempt,
            session_duration_min: stats_report.session_report.session_duration_min,
            disconnection_time_ms: stats_report.session_report.disconnection_time_ms,
            exit_id: stats_report.session_report.exit_id.clone(),
            follow_up_id: stats_report.session_report.follow_up_id.clone(),
            error: stats_report.session_report.error.clone(),
        }
    }
}
