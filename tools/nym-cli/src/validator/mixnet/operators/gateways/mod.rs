// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    gateway: nym_cli_commands::validator::mixnet::operators::gateway::MixnetOperatorsGateway,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match gateway.command {
        nym_cli_commands::validator::mixnet::operators::gateway::MixnetOperatorsGatewayCommands::Bond(args) => {
            nym_cli_commands::validator::mixnet::operators::gateway::bond_gateway::bond_gateway(args, create_signing_client(global_args, network_details)?).await
        },
        nym_cli_commands::validator::mixnet::operators::gateway::MixnetOperatorsGatewayCommands::Unbound(_args) => {
            nym_cli_commands::validator::mixnet::operators::gateway::unbond_gateway::unbond_gateway(create_signing_client(global_args, network_details)?).await
        },
        _ => unreachable!(),
    }
    Ok(())
}
