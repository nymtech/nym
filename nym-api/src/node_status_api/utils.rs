// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::Uptime;
use crate::node_status_api::{FIFTEEN_MINUTES, ONE_HOUR};
use crate::storage::models::NodeStatus;
use log::warn;
use nym_mixnet_contract_common::MixId;

use time::OffsetDateTime;

// A temporary helper structs used to produce reports for active nodes.
pub(crate) struct ActiveMixnodeStatuses {
    pub(crate) mix_id: MixId,

    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) statuses: Vec<NodeStatus>,
}

pub(crate) struct ActiveGatewayStatuses {
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) statuses: Vec<NodeStatus>,
}

// A helper intermediate struct to remove duplicate code for construction of mixnode and gateway reports
pub(crate) struct NodeUptimes {
    pub(crate) most_recent: Uptime,

    pub(crate) last_hour: Uptime,
    pub(crate) last_day: Uptime,
}

impl NodeUptimes {
    pub(crate) fn calculate_from_last_day_reports(
        report_time: OffsetDateTime,
        last_day: Vec<NodeStatus>,
        last_hour_test_runs: usize,
        last_day_test_runs: usize,
    ) -> Self {
        let hour_ago = (report_time - ONE_HOUR).unix_timestamp();
        let fifteen_minutes_ago = (report_time - FIFTEEN_MINUTES).unix_timestamp();

        // If somehow we have more reports than the actual test runs it means something weird is going on
        // (or we just started running this code on old data, so if it appears for first 24h, it's fine and actually expected
        // as we would not have any run information from the past)
        // Either way, bound the the number of "up" reports by number of test runs and log warnings
        // if that happens

        let last_day_sum: f32 = if last_day.len() > last_day_test_runs {
            warn!(
                "We have more reports than the actual number of test runs in last day! ({} reports for {} test runs)",
                last_day.len(),
                last_day_test_runs,
            );
            last_day
                .iter()
                .take(last_day_test_runs)
                .map(|report| report.reliability() as f32)
                .sum()
        } else {
            // we average over expected number of test runs so if a node was not online for some of them
            // it's treated as if it had a "zero" status.
            last_day
                .iter()
                .map(|report| report.reliability() as f32)
                .sum()
        };

        let last_hour_reports = last_day
            .iter()
            .filter(|report| report.timestamp() >= hour_ago)
            .count();

        let last_hour_sum: f32 = if last_hour_reports > last_hour_test_runs {
            warn!(
                "We have more reports than the actual number of test runs in last hour! ({} reports for {} test runs)",
                last_hour_reports,
                last_hour_test_runs,
            );
            last_day
                .iter()
                .filter(|report| report.timestamp() >= hour_ago)
                .take(last_hour_test_runs)
                .map(|report| report.reliability() as f32)
                .sum()
        } else {
            last_day
                .iter()
                .filter(|report| report.timestamp() >= hour_ago)
                .map(|report| report.reliability() as f32)
                .sum()
        };

        // find the most recent
        let most_recent_report = last_day.iter().max_by_key(|report| report.timestamp());

        let most_recent = if let Some(most_recent_report) = most_recent_report {
            // make sure its within last 15min
            if most_recent_report.timestamp() >= fifteen_minutes_ago {
                most_recent_report.reliability()
            } else {
                0
            }
        } else {
            0
        };

        // the unwraps in Uptime::from_ratio are fine because it's impossible for us to have more "up" results
        // than total test runs as we just bounded them
        NodeUptimes {
            most_recent: most_recent.try_into().unwrap(),
            last_hour: Uptime::from_uptime_sum(last_hour_sum, last_hour_test_runs).unwrap(),
            last_day: Uptime::from_uptime_sum(last_day_sum, last_day_test_runs).unwrap(),
        }
    }
}
