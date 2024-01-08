// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod update_config;
pub mod update_cost_params;
pub mod vesting_update_config;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsMixnodeSettings {
    #[clap(subcommand)]
    pub command: MixnetOperatorsMixnodeSettingsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsMixnodeSettingsCommands {
    /// Update mixnode configuration
    UpdateConfig(update_config::Args),
    /// Update mixnode configuration for a mixnode bonded with locked tokens
    VestingUpdateConfig(vesting_update_config::Args),
    /// Update mixnode cost parameters
    UpdateCostParameters(update_cost_params::Args),
}
