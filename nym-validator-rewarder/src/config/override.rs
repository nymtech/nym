// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::ConfigOverridableArgs;
use crate::config::Config;

pub trait ConfigOverride {
    fn override_config(self, config: &mut Config);
}

impl ConfigOverride for ConfigOverridableArgs {
    fn override_config(self, config: &mut Config) {
        if self.disable_block_signing_rewarding {
            config.block_signing.enabled = false
        }

        if self.block_signing_monitoring_only {
            config.block_signing.monitor_only = true
        }

        if self.ticketbook_issuance_monitoring_only {
            config.ticketbook_issuance.monitor_only = true
        }

        if self.disable_ticketbook_issuance_rewarding {
            config.ticketbook_issuance.enabled = false
        }

        if let Some(scraper_endpoint) = self.scraper_endpoint {
            config.nyxd_scraper.websocket_url = scraper_endpoint
        }

        if let Some(nyxd_endpoint) = self.nyxd_endpoint {
            config.base.upstream_nyxd = nyxd_endpoint
        }

        if let Some(epoch_budget) = self.epoch_budget {
            config.rewarding.daily_budget = epoch_budget
        }

        if let Some(block_signing_reward_ratio) = self.block_signing_reward_ratio {
            config.rewarding.ratios.block_signing = block_signing_reward_ratio;
        }

        if let Some(ticketbook_issuance_reward_ratio) = self.ticketbook_issuance_reward_ratio {
            config.rewarding.ratios.ticketbook_issuance = ticketbook_issuance_reward_ratio;
        }
    }
}
