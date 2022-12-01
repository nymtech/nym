// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod mixnet;
pub mod vesting;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct GenerateMessage {
    #[clap(subcommand)]
    pub command: GenerateMessageCommands,
}

#[derive(Debug, Subcommand)]
pub enum GenerateMessageCommands {
    Mixnet(mixnet::Args),
    Vesting(vesting::Args),
}
