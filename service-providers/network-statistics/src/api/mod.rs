// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use rocket::{Ignite, Rocket};

use crate::storage::NetworkStatisticsStorage;
use error::Result;
use routes::{post_all_statistics, post_statistic};

use statistics_common::api::STATISTICS_SERVICE_VERSION;

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
        let shutdown_handle = self.rocket.shutdown();
        tokio::spawn(self.rocket.launch());

        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        info!("Received SIGINT - the network statistics API will terminate now");
        shutdown_handle.notify();
    }
}
