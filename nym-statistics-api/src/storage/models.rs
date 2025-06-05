use axum_extra::headers::UserAgent;
use celes::Country;
use nym_statistics_common::report::vpn_client::VpnClientStatsReport;
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
        received_from: String,
        maybe_country: Option<Country>,
        from_mixnet: bool,
    ) -> Option<Self> {
        stats_report.basic_usage.as_ref().map(|usage_report| Self {
            received_at,
            received_from,
            connection_time_ms: usage_report.connection_time_ms,
            two_hop: usage_report.two_hop,
            country_code: maybe_country.map(|c| c.alpha2.into()),
            from_mixnet,
        })
    }
}
