// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod balance;
pub mod create_vesting_schedule;
pub mod query_vesting_schedule;
pub mod withdraw_vested;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct VestingSchedule {
    #[clap(subcommand)]
    pub command: Option<VestingScheduleCommands>,
}

#[derive(Debug, Subcommand)]
pub enum VestingScheduleCommands {
    /// Creates a vesting schedule
    Create(crate::validator::vesting::create_vesting_schedule::Args),
    /// Query for vesting schedule
    Query(crate::validator::vesting::query_vesting_schedule::Args),
    /// Get the amount that has vested and is free for withdrawal, delegation or bonding
    VestedBalance(crate::validator::vesting::balance::Args),
    /// Withdraw vested tokens (note: the available amount excludes anything delegated or bonded before or after vesting)
    WithdrawVested(crate::validator::vesting::withdraw_vested::Args),
}
