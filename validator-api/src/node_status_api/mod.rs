// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::fairing::AdHoc;
use std::time::Duration;

pub(crate) mod local_guard;
pub(crate) mod models;
pub(crate) mod routes;
pub(crate) mod uptime_updater;
pub(crate) mod utils;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

pub(crate) fn stage_full() -> AdHoc {
    AdHoc::on_ignite("Node Status API Stage", |rocket| async {
        rocket.mount(
            "/v1/status",
            routes![
                routes::mixnode_report,
                routes::gateway_report,
                routes::mixnode_uptime_history,
                routes::gateway_uptime_history,
                routes::mixnode_core_status_count,
                routes::gateway_core_status_count,
                routes::get_mixnode_status,
                routes::get_mixnode_reward_estimation,
                routes::get_mixnode_stake_saturation,
                routes::get_mixnode_inclusion_probability,
                routes::get_mixnode_avg_uptime,
                routes::get_mixnode_avg_uptimes,
            ],
        )
    })
}

// in the minimal variant we would not have access to endpoints relying on existence
// of the network monitor and the associated storage
pub(crate) fn stage_minimal() -> AdHoc {
    AdHoc::on_ignite("Node Status API Stage", |rocket| async {
        rocket.mount(
            "/v1/status",
            routes![
                routes::get_mixnode_status,
                routes::get_mixnode_stake_saturation,
                routes::get_mixnode_inclusion_probability,
            ],
        )
    })
}
