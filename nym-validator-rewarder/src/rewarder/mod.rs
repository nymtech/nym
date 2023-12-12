// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use nym_network_defaults::NymNetworkDetails;
use nym_task::TaskManager;
use nyxd_scraper::NyxdScraper;
use std::time::Duration;
use tracing::info;

mod tasks;

pub struct Rewarder {
    config: Config,
    nyxd_scraper: NyxdScraper,
}

impl Rewarder {
    pub async fn new(config: Config) -> Result<Self, NymRewarderError> {
        let nyxd_scraper = NyxdScraper::new(config.scraper_config()).await?;

        Ok(Rewarder {
            config,
            nyxd_scraper,
        })
    }

    pub async fn run(mut self) -> Result<(), NymRewarderError> {
        info!("Starting nym validators rewarder");

        // setup shutdowns
        let task_manager = TaskManager::new(5);

        self.nyxd_scraper.start().await?;

        tokio::time::sleep(Duration::from_secs(30)).await;

        /*
           task 1:
           on timer:
               - go to DKG contract
               - get all coconut signers
               - for each of them get the info, verify, etc

           task 2:
           on timer (or maybe per block?):
               - query abci endpoint for VP
               - also maybe missed blocks, etc

        */

        self.nyxd_scraper.stop().await;

        todo!()
    }
}
