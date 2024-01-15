// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use rocket::{Ignite, Rocket};

use crate::storage::NetworkStatisticsStorage;
use error::Result;
use routes::{post_all_statistics, post_statistic};

use nym_statistics_common::api::STATISTICS_SERVICE_VERSION;
use nym_task::TaskManager;

mod error;
mod routes;

pub(crate) struct NetworkStatisticsAPI {
    rocket: Rocket<Ignite>,
}

impl NetworkStatisticsAPI {
    pub async fn init(storage: NetworkStatisticsStorage) -> Result<Self> {
        let rocket = rocket::build()
            .mount(
                STATISTICS_SERVICE_VERSION,
                rocket::routes![post_all_statistics, post_statistic],
            )
            .manage(storage.clone())
            .ignite()
            .await
            .map_err(Box::new)?;

        Ok(NetworkStatisticsAPI { rocket })
    }

    pub async fn run(self) {
        let rocket_shutdown_handle = self.rocket.shutdown();
        let mut shutdown = TaskManager::new(10);
        tokio::spawn(self.rocket.launch());

        shutdown.catch_interrupt().await.ok();
        info!("Stopping network statistics");
        rocket_shutdown_handle.notify();
    }
}
