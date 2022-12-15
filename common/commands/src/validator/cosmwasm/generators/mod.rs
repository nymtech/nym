// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod coconut_bandwidth;
pub mod coconut_dkg;
pub mod mixnet;
pub mod multisig;
pub mod vesting;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct GenerateMessage {
    #[clap(subcommand)]
    pub command: GenerateMessageCommands,
}

#[derive(Debug, Subcommand)]
pub enum GenerateMessageCommands {
    CoconutBandwidth(coconut_bandwidth::Args),
    CoconutDKG(coconut_dkg::Args),
    Mixnet(mixnet::Args),
    Multisig(multisig::Args),
    Vesting(vesting::Args),
}
