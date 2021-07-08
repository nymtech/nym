// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::fairing::AdHoc;
use std::time::Duration;

pub(crate) mod models;
pub(crate) mod routes;
pub(crate) mod storage;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

pub(crate) fn stage() -> AdHoc {
    AdHoc::on_ignite("SQLx Stage", |rocket| async {
        rocket
            .attach(storage::NodeStatusStorage::stage())
            .mount("/v1/status", routes![routes::mixnode_report])
    })
}
