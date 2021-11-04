// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage;
use rocket::fairing::AdHoc;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) mod local_guard;
pub(crate) mod models;
pub(crate) mod routes;
pub(crate) mod uptime_updater;
pub(crate) mod utils;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

pub(crate) fn stage(database_path: PathBuf) -> AdHoc {
    AdHoc::on_ignite("SQLx Stage", |rocket| async {
        rocket
            .attach(storage::ValidatorApiStorage::stage(database_path))
            .mount(
                "/v1/status",
                routes![
                    routes::mixnode_report,
                    routes::gateway_report,
                    routes::mixnode_uptime_history,
                    routes::gateway_uptime_history,
                ],
            )
    })
}
