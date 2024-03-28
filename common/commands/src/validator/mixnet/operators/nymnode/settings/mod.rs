// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod update_config;
pub mod update_cost_params;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsNymNodeSettings {
    #[clap(subcommand)]
    pub command: MixnetOperatorsNymNodeSettingsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsNymNodeSettingsCommands {
    /// Update Nym Node configuration
    UpdateConfig(update_config::Args),
    /// Update Nym Node cost parameters
    UpdateCostParameters(update_cost_params::Args),
}
