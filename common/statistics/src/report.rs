// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::error::StatsError;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Report object containing both data to be reported and client / device context. We take extra care not to overcapture context information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientStatsReport {
    last_update_time: OffsetDateTime,
    os_information: os_info::Info,
    connection_time: Duration,
}

impl Default for ClientStatsReport {
    fn default() -> Self {
        ClientStatsReport {
            //Safety : 0 is always a valid number of seconds
            #[allow(clippy::unwrap_used)]
            last_update_time: OffsetDateTime::now_utc().replace_second(0).unwrap(), // allow a bigger anonymity set wrt to reports
            os_information: os_info::get(), //SW is this revealing too much info?
            connection_time: Default::default(),
        }
    }
}

impl TryFrom<ClientStatsReport> for Vec<u8> {
    type Error = StatsError;

    fn try_from(value: ClientStatsReport) -> Result<Self, Self::Error> {
        let report_json = serde_json::to_string(&value)?;
        Ok(report_json.as_bytes().to_vec())
    }
}

impl TryFrom<Vec<u8>> for ClientStatsReport {
    type Error = StatsError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let report_str = String::from_utf8(value)
            .map_err(|err| StatsError::ReportBytesDeserialization(err.to_string()))?;
        Ok(serde_json::from_str(&report_str)?)
    }
}

/// This trait represents objects that can be reported by the metrics controller and
/// provides the function by which they will be called to report their metrics.
pub trait StatisticsReporter {
    /// Marshall the metrics into a string and write them to the provided formatter.
    fn marshall(&self) -> std::io::Result<String>;
}
