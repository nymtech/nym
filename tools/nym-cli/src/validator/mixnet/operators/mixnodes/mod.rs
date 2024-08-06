// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod families;
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
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Families(families) => {
            families::execute(global_args, families, network_details).await?
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Bond(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::bond_mixnode::bond_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::Unbond(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::unbond_mixnode::unbond_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::CreateMixnodeBondingSignPayload(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::mixnode_bonding_sign_payload::create_payload(args,create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::PledgeMore(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::pledge_more::pledge_more(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::PledgeMoreVesting(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::vesting_pledge_more::vesting_pledge_more(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::DecreasePledge(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::decrease_pledge::decrease_pledge(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::DecreasePledgeVesting(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::vesting_decrease_pledge::vesting_decrease_pledge(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::mixnode::MixnetOperatorsMixnodeCommands::MigrateVestedNode(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::migrate_vested_mixnode::migrate_vested_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        _ => unreachable!()
    }
    Ok(())
}
