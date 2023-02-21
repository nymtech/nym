// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod keys;
pub(crate) mod rewards;
pub(crate) mod settings;

pub(crate) async fn execute(
    global_args: ClientArgs,
    mixnode: nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnode,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match mixnode.command {
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Keys(keys) => {
            keys::execute(keys).await?
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Rewards(rewards) => {
            rewards::execute(global_args, rewards, network_details).await?
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Settings(settings) => {
            settings::execute(global_args, settings, network_details).await?
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Bond(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::bond_mixnode::bond_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Unbound(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::unbond_mixnode::unbond_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        _ => unreachable!(),
    }
    Ok(())
}
