// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::clients::{
    gateway_conn_statistics::GatewayStats, nym_api_statistics::NymApiStats,
    packet_statistics::PacketStatistics,
};

use super::error::StatsError;

use serde::{Deserialize, Serialize};
use sysinfo::System;
use time::OffsetDateTime;

/// Report object containing both data to be reported and client / device context. We take extra care not to overcapture context information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientStatsReport {
    pub(crate) last_update_time: OffsetDateTime,
    pub(crate) client_id: String,
    pub(crate) client_type: String,
    pub(crate) os_information: OsInformation,
    pub(crate) packet_stats: PacketStatistics,
    pub(crate) gateway_conn_stats: GatewayStats,
    pub(crate) nym_api_stats: NymApiStats,
}

impl From<ClientStatsReport> for Vec<u8> {
    fn from(value: ClientStatsReport) -> Self {
        // safety, no custom serialisation
        #[allow(clippy::unwrap_used)]
        let report_json = serde_json::to_string(&value).unwrap();
        report_json.as_bytes().to_vec()
    }
}

impl TryFrom<&[u8]> for ClientStatsReport {
    type Error = StatsError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(value)?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsInformation {
    pub(crate) os_type: String,
    pub(crate) os_version: Option<String>,
    pub(crate) os_arch: Option<String>,
}

impl OsInformation {
    pub fn new() -> Self {
        OsInformation {
            os_type: System::distribution_id(),
            os_version: System::long_os_version(),
            os_arch: System::cpu_arch(),
        }
    }
}

impl Default for OsInformation {
    fn default() -> Self {
        Self::new()
    }
}
