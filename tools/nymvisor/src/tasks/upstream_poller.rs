// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::NymvisorError;
use crate::upgrades::types::{UpgradeInfo, UpgradePlan};
use reqwest::get;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

pub(crate) struct UpstreamPoller {
    config: Config,
}

impl UpstreamPoller {
    pub(crate) fn new(config: &Config) -> Self {
        UpstreamPoller {
            config: config.clone(),
        }
    }

    /// Poll the upstream url to see if new upgrade has been published.
    /// If so, save it to `upgrade-info.json` and update the `upgrade-plan.json`
    async fn check_upstream(&self) -> Result<(), NymvisorError> {
        let upgrade_info: UpgradeInfo = get(self.config.upstream_upgrade_url())
            .await
            .map_err(|source| NymvisorError::UpstreamQueryFailure {
                url: self.config.upstream_upgrade_url(),
                source,
            })?
            .json()
            .await
            .map_err(|source| NymvisorError::UpstreamQueryFailure {
                url: self.config.upstream_upgrade_url(),
                source,
            })?;

        debug!("obtained the following upgrade info: {upgrade_info:?}");

        let mut plan = UpgradePlan::try_load(self.config.upgrade_plan_filepath())?;

        // if the current version is the same as the one announced by upstream, we're done
        if upgrade_info.version == plan.current().version {
            debug!("the upstream version is the same one as the currently used");
            return Ok(());
        }

        if !plan.has_planned(&upgrade_info) {
            if let Err(err) =
                upgrade_info.save(self.config.upgrade_info_filepath(&upgrade_info.name))
            {
                error!("failed to save new upgrade info: {err}");
                return Err(err);
            }
            debug!("saved new upgrade info");

            if let Err(err) = plan.insert_new_upgrade(upgrade_info) {
                error!("failed to insert new upgrade info into the current upgrade plan: {err}");
                return Err(err);
            }
            debug!("inserted new upgrade info into the upgrade plan");
        }

        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        let mut interval = tokio::time::interval(self.config.nymvisor.debug.upstream_polling_rate);
        loop {
            // note: first tick happens immediately
            interval.tick().await;
            if let Err(err) = self.check_upstream().await {
                warn!("failed to check the upstream for new upgrade information: {err}. we will try to poll it again in {}", humantime::format_duration(interval.period()));
            }
        }
    }

    pub(crate) fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
