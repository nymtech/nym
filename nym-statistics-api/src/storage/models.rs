use std::net::IpAddr;

use axum_extra::headers::UserAgent;
use celes::Country;
use nym_statistics_common::report::vpn_client::{VpnClientStatsReport, VpnSessionReport};
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
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct ConnectionInfoDto {
    pub(crate) received_at: OffsetDateTime,
    pub(crate) received_from: String,
    pub(crate) connection_time_ms: Option<i32>,
    pub(crate) two_hop: bool,
    pub(crate) country_code: Option<String>,
    pub(crate) from_mixnet: bool,
}

impl ConnectionInfoDto {
    pub(crate) fn maybe_new(
        received_at: OffsetDateTime,
        stats_report: &VpnClientStatsReport,
        received_from: IpAddr,
        maybe_country: Option<Country>,
        from_mixnet: bool,
    ) -> Option<Self> {
        stats_report.basic_usage.as_ref().map(|usage_report| Self {
            received_at,
            received_from: received_from.to_string(),
            connection_time_ms: usage_report.connection_time_ms,
            two_hop: usage_report.two_hop,
            country_code: maybe_country.map(|c| c.alpha2.into()),
            from_mixnet,
        })
    }
}

// New structure. The two above will be removed when it is confirmed to work
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
pub(crate) struct SessionInfoDto {
    pub received_at: OffsetDateTime,
    pub day: Date,
    pub connection_time_ms: i32,
    pub session_duration_min: i32,
    pub two_hop: bool,
    pub exit_id: String,
    pub error: Option<String>,
    pub country_code: Option<String>,
    pub from_mixnet: bool,
}

impl SessionInfoDto {
    pub(crate) fn new(
        received_at: OffsetDateTime,
        session_report: &VpnSessionReport,
        maybe_country: Option<Country>,
        from_mixnet: bool,
    ) -> Self {
        Self {
            received_at,
            day: session_report.day,
            connection_time_ms: session_report.connection_time_ms,
            session_duration_min: session_report.session_duration_min,
            two_hop: session_report.two_hop,
            exit_id: session_report.exit_id.clone(),
            error: session_report.error.clone(),
            country_code: maybe_country.map(|c| c.alpha2.into()),
            from_mixnet,
        }
    }
}
