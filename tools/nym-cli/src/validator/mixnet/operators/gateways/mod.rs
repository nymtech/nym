// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::gateway::{
    MixnetOperatorsGateway, MixnetOperatorsGatewayCommands,
};
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod settings;

pub(crate) async fn execute(
    global_args: ClientArgs,
    gateway: MixnetOperatorsGateway,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match gateway.command {
        MixnetOperatorsGatewayCommands::Unbond(_args) => {
            nym_cli_commands::validator::mixnet::operators::gateway::unbond_gateway::unbond_gateway(create_signing_client(global_args, network_details)?).await
        },
        MixnetOperatorsGatewayCommands::MigrateToNymnode(args) => nym_cli_commands::validator::mixnet::operators::gateway::nymnode_migration::migrate_to_nymnode(args, create_signing_client(global_args, network_details)?).await,
        _ => bail!("this command is no longer available. please migrate your mixnode into a Nym-Node via `migrate-to-nymnode` command")
    }
    Ok(())
}
