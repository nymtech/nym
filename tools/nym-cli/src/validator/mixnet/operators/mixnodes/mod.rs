// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands;
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
        MixnetOperatorsMixnodeCommands::Unbond(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::unbond_mixnode::unbond_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        MixnetOperatorsMixnodeCommands::MigrateVestedNode(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::migrate_vested_mixnode::migrate_vested_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        MixnetOperatorsMixnodeCommands::MigrateToNymnode(args) => nym_cli_commands::validator::mixnet::operators::mixnode::nymnode_migration::migrate_to_nymnode(args, create_signing_client(global_args, network_details)?).await,
        _ => bail!("this command is no longer available. please migrate your mixnode into a Nym-Node via `migrate-to-nymnode` command")
    }
    Ok(())
}
