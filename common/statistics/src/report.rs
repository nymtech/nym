// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::clients::{
    connection::ConnectionStats, gateway_conn_statistics::GatewayStats,
    nym_api_statistics::NymApiStats, packet_statistics::PacketStatistics,
};

use super::error::StatsError;

use serde::{Deserialize, Serialize};
use sysinfo::System;
use time::OffsetDateTime;

const KIND: &str = "client_stats_report";
const VERSION: &str = "v1";

/// Report object containing both data to be reported and client / device context. We take extra care not to overcapture context information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientStatsReport {
    pub kind: String,
    pub api_version: String,
    pub last_update_time: OffsetDateTime,
    pub client_id: String,
    pub client_type: String,
    pub os_information: OsInformation,
    pub packet_stats: PacketStatistics,
    pub gateway_conn_stats: GatewayStats,
    pub nym_api_stats: NymApiStats,
    pub connection_stats: ConnectionStats,
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

impl Default for ClientStatsReport {
    fn default() -> Self {
        ClientStatsReport {
            kind: KIND.to_string(),
            api_version: VERSION.to_string(),
            last_update_time: OffsetDateTime::now_utc(),
            client_id: Default::default(),
            client_type: Default::default(),
            os_information: Default::default(),
            packet_stats: Default::default(),
            gateway_conn_stats: Default::default(),
            nym_api_stats: Default::default(),
            connection_stats: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsInformation {
    pub os_type: String,
    pub os_version: Option<String>,
    pub os_arch: String,
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
