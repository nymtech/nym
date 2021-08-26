// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::Uptime;
use crate::node_status_api::{FIFTEEN_MINUTES, ONE_HOUR};
use crate::storage::models::NodeStatus;
use log::warn;
use std::cmp::min;
use time::OffsetDateTime;

// A temporary helper struct used to produce reports for active nodes.
pub(crate) struct ActiveNodeDayStatuses {
    pub(crate) identity: String,
    pub(crate) owner: String,
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
        report_time: OffsetDateTime,
        last_day_ipv4: Vec<NodeStatus>,
        last_day_ipv6: Vec<NodeStatus>,
        last_hour_test_runs: usize,
        last_day_test_runs: usize,
    ) -> Self {
        let hour_ago = (report_time - ONE_HOUR).unix_timestamp();
        let fifteen_minutes_ago = (report_time - FIFTEEN_MINUTES).unix_timestamp();

        let mut ipv4_day_up = last_day_ipv4.iter().filter(|report| report.up).count();
        let mut ipv6_day_up = last_day_ipv6.iter().filter(|report| report.up).count();

        let mut ipv4_hour_up = last_day_ipv4
            .iter()
            .filter(|report| report.up && report.timestamp >= hour_ago)
            .count();
        let mut ipv6_hour_up = last_day_ipv6
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

        // If somehow we have more "up" reports than the actual test runs it means something weird is going on
        // (or we just started running this code on old data, so if it appears for first 24h, it's fine and actually expected
        // as we would not have any run information from the past)
        // Either way, bound the the number of "up" reports by number of test runs and log warnings
        // if that happens
        if ipv4_hour_up > last_hour_test_runs || ipv6_hour_up > last_hour_test_runs {
            warn!(
                "We have more 'up' reports than the actual number of test runs in last hour! ({} ipv4 'ups', {} ipv6 'ups' for {} test runs)",
                ipv4_hour_up,
                ipv6_hour_up,
                last_hour_test_runs,
            );
            ipv4_hour_up = min(ipv4_hour_up, last_hour_test_runs);
            ipv6_hour_up = min(ipv6_hour_up, last_hour_test_runs);
        }

        if ipv4_day_up > last_day_test_runs || ipv6_day_up > last_day_test_runs {
            warn!(
                "We have more 'up' reports than the actual number of test runs in last day! ({} ipv4 'ups', {} ipv6 'ups' for {} test runs)",
                ipv4_day_up,
                ipv6_day_up,
                last_day_test_runs,
            );
            ipv4_day_up = min(ipv4_day_up, last_day_test_runs);
            ipv6_day_up = min(ipv6_day_up, last_day_test_runs);
        }

        // the unwraps in Uptime::from_ratio are fine because it's impossible for us to have more "up" results
        // than total test runs as we just bounded them
        NodeUptimes {
            most_recent_ipv4,
            most_recent_ipv6,
            last_hour_ipv4: Uptime::from_ratio(ipv4_hour_up, last_hour_test_runs).unwrap(),
            last_hour_ipv6: Uptime::from_ratio(ipv6_hour_up, last_hour_test_runs).unwrap(),
            last_day_ipv4: Uptime::from_ratio(ipv4_day_up, last_day_test_runs).unwrap(),
            last_day_ipv6: Uptime::from_ratio(ipv6_day_up, last_day_test_runs).unwrap(),
        }
    }
}
