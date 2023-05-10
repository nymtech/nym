// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod gateway;
pub mod mixnode;
pub mod name;
pub mod service;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperators {
    #[clap(subcommand)]
    pub command: MixnetOperatorsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsCommands {
    /// Manage your mixnode
    Mixnode(mixnode::MixnetOperatorsMixnode),
    /// Manage your gateway
    Gateway(gateway::MixnetOperatorsGateway),
    /// Manage your service
    ServiceProvider(service::MixnetOperatorsService),
    /// Manage your registered name
    Name(name::MixnetOperatorsName),
}
