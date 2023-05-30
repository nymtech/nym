// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod query_all_gateways;
pub mod query_all_mixnodes;
pub mod query_all_names;
pub mod query_all_service_providers;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetQuery {
    #[clap(subcommand)]
    pub command: MixnetQueryCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetQueryCommands {
    /// Query mixnodes
    Mixnodes(query_all_mixnodes::Args),
    /// Query gateways
    Gateways(query_all_gateways::Args),
    /// Query announced service-providers
    ServiceProviders(query_all_service_providers::Args),
    /// Query registed names
    Names(query_all_names::Args),
}
