// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::ClientArgs;
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod gateways;
pub(crate) mod mixnodes;
pub(crate) mod name;
pub(crate) mod services;

pub(crate) async fn execute(
    global_args: ClientArgs,
    operators: nym_cli_commands::validator::mixnet::operators::MixnetOperators,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match operators.command {
        nym_cli_commands::validator::mixnet::operators::MixnetOperatorsCommands::Gateway(
            gateway,
        ) => gateways::execute(global_args, gateway, network_details).await?,
        nym_cli_commands::validator::mixnet::operators::MixnetOperatorsCommands::Mixnode(
            mixnode,
        ) => mixnodes::execute(global_args, mixnode, network_details).await?,
        nym_cli_commands::validator::mixnet::operators::MixnetOperatorsCommands::ServiceProvider(service) => services::execute(global_args, service, network_details).await?,
        nym_cli_commands::validator::mixnet::operators::MixnetOperatorsCommands::Name(name) => name::execute(global_args, name, network_details).await?,
    }
    Ok(())
}
