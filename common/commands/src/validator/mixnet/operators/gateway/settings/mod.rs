// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod update_config;
pub mod vesting_update_config;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsGatewaySettings {
    #[clap(subcommand)]
    pub command: MixnetOperatorsGatewaySettingsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsGatewaySettingsCommands {
    /// Update gateway configuration
    UpdateConfig(update_config::Args),
    /// Update gateway configuration for a gateway bonded with locked tokens
    VestingUpdateConfig(vesting_update_config::Args),
}
