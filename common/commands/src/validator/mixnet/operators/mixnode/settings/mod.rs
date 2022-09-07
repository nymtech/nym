// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod update_profit_percent;
pub mod vesting_update_profit_percent;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsMixnodeSettings {
    #[clap(subcommand)]
    pub command: MixnetOperatorsMixnodeSettingsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsMixnodeSettingsCommands {
    /// Update profit percentage
    UpdateProfitPercentage(update_profit_percent::Args),
    /// Update profit percentage for a mixnode bonded with locked tokens
    VestingUpdateProfitPercentage(vesting_update_profit_percent::Args),
}
