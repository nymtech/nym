// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::Uptime;
use crate::node_status_api::{FIFTEEN_MINUTES, ONE_HOUR};
use sqlx::types::time::OffsetDateTime;

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct NodeStatus {
    pub(crate) timestamp: i64,
    pub(crate) up: bool,
}

// A temporary helper struct used to produce reports for active nodes.
pub(crate) struct ActiveNodeDayStatuses {
    pub(crate) pub_key: String,
    pub(crate) owner: String,
    pub(crate) node_id: i64,

    pub(crate) ipv4_statuses: Vec<NodeStatus>,
    pub(crate) ipv6_statuses: Vec<NodeStatus>,
}

// A helper intermediate struct to remove duplicate code for construction of mixnode and gateway reports
pub(crate) struct NodeUptimes {
    pub(crate) most_recent_ipv4: bool,
    pub(crate) most_recent_ipv6: bool,

    pub(crate) last_hour_ipv4: Uptime,
    pub(crate) last_hour_ipv6: Uptime,

    pub(crate) last_day_ipv4: Uptime,
    pub(crate) last_day_ipv6: Uptime,
}

impl NodeUptimes {
    pub(crate) fn calculate_from_last_day_reports(
        last_day_ipv4: Vec<NodeStatus>,
        last_day_ipv6: Vec<NodeStatus>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();
        let fifteen_minutes_ago = (now - FIFTEEN_MINUTES).unix_timestamp();

        let ipv4_day_total = last_day_ipv4.len();
        let ipv6_day_total = last_day_ipv6.len();

        let ipv4_day_up = last_day_ipv4.iter().filter(|report| report.up).count();
        let ipv6_day_up = last_day_ipv6.iter().filter(|report| report.up).count();

        let ipv4_hour_total = last_day_ipv4
            .iter()
            .filter(|report| report.timestamp >= hour_ago)
            .count();
        let ipv6_hour_total = last_day_ipv6
            .iter()
            .filter(|report| report.timestamp >= hour_ago)
            .count();

        let ipv4_hour_up = last_day_ipv4
            .iter()
            .filter(|report| report.up && report.timestamp >= hour_ago)
            .count();
        let ipv6_hour_up = last_day_ipv6
            .iter()
            .filter(|report| report.up && report.timestamp >= hour_ago)
            .count();

        // most recent status MUST BE within last 15min
        let most_recent_ipv4 = last_day_ipv4
            .iter()
            .max_by_key(|report| report.timestamp) // find the most recent
            .map(|status| status.timestamp >= fifteen_minutes_ago && status.up) // make sure its within last 15min
            .unwrap_or_default();
        let most_recent_ipv6 = last_day_ipv6
            .iter()
            .max_by_key(|report| report.timestamp) // find the most recent
            .map(|status| status.timestamp >= fifteen_minutes_ago && status.up) // make sure its within last 15min
            .unwrap_or_default();

        // the unwraps in Uptime::from_ratio are fine because it's impossible for us to have more "up" results than all results in total
        // because both of those values originate from the same vector
        NodeUptimes {
            most_recent_ipv4,
            most_recent_ipv6,
            last_hour_ipv4: Uptime::from_ratio(ipv4_hour_up, ipv4_hour_total).unwrap(),
            last_hour_ipv6: Uptime::from_ratio(ipv6_hour_up, ipv6_hour_total).unwrap(),
            last_day_ipv4: Uptime::from_ratio(ipv4_day_up, ipv4_day_total).unwrap(),
            last_day_ipv6: Uptime::from_ratio(ipv6_day_up, ipv6_day_total).unwrap(),
        }
    }
}
