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

        if self.disable_block_scraper {
            config.nyxd_scraper.enabled = false
        }

        if self.disable_credential_issuance_rewarding {
            config.issuance_monitor.enabled = false
        }

        if let Some(credential_monitor_run_interval) = self.credential_monitor_run_interval {
            config.issuance_monitor.run_interval = credential_monitor_run_interval.into()
        }

        if let Some(credential_monitor_min_validation) = self.credential_monitor_min_validation {
            config.issuance_monitor.min_validate_per_issuer = credential_monitor_min_validation
        }

        if let Some(credential_monitor_sampling_rate) = self.credential_monitor_sampling_rate {
            config.issuance_monitor.sampling_rate = credential_monitor_sampling_rate
        }

        if let Some(scraper_endpoint) = self.scraper_endpoint {
            config.nyxd_scraper.websocket_url = scraper_endpoint
        }

        if let Some(nyxd_endpoint) = self.nyxd_endpoint {
            config.base.upstream_nyxd = nyxd_endpoint
        }

        if let Some(epoch_budget) = self.epoch_budget {
            config.rewarding.epoch_budget = epoch_budget
        }

        if let Some(epoch_duration_secs) = self.epoch_duration {
            config.rewarding.epoch_duration = epoch_duration_secs.into()
        }

        if let Some(block_signing_reward_ratio) = self.block_signing_reward_ratio {
            config.rewarding.ratios.block_signing = block_signing_reward_ratio;
        }

        if let Some(credential_issuance_reward_ratio) = self.credential_issuance_reward_ratio {
            config.rewarding.ratios.credential_issuance = credential_issuance_reward_ratio;
        }

        if let Some(credential_verification_reward_ratio) =
            self.credential_verification_reward_ratio
        {
            config.rewarding.ratios.credential_verification = credential_verification_reward_ratio;
        }
    }
}
