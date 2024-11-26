// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::nymnode::bond_nymnode::bond_nymnode;
use nym_cli_commands::validator::mixnet::operators::nymnode::unbond_nymnode::unbond_nymnode;
use nym_cli_commands::validator::mixnet::operators::nymnode::{
    nymnode_bonding_sign_payload, MixnetOperatorsNymNodeCommands,
};
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod keys;
pub(crate) mod pledge;
pub(crate) mod rewards;
pub(crate) mod settings;

pub(crate) async fn execute(
    global_args: ClientArgs,
    nymnode: nym_cli_commands::validator::mixnet::operators::nymnode::MixnetOperatorsNymNode,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match nymnode.command {
        MixnetOperatorsNymNodeCommands::CreateNodeBondingSignPayload(args) => {
            nymnode_bonding_sign_payload::create_payload(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        MixnetOperatorsNymNodeCommands::Keys(keys) => keys::execute(keys).await?,
        MixnetOperatorsNymNodeCommands::Rewards(rewards) => {
            rewards::execute(global_args, rewards, network_details).await?
        }
        MixnetOperatorsNymNodeCommands::Settings(settings) => {
            settings::execute(global_args, settings, network_details).await?
        }
        MixnetOperatorsNymNodeCommands::Pledge(pledge) => {
            pledge::execute(global_args, pledge, network_details).await?
        }
        MixnetOperatorsNymNodeCommands::Bond(args) => {
            bond_nymnode(args, create_signing_client(global_args, network_details)?).await
        }
        MixnetOperatorsNymNodeCommands::Unbond(args) => {
            unbond_nymnode(args, create_signing_client(global_args, network_details)?).await
        }
    }

    Ok(())
}
