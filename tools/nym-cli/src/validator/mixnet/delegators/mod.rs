// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{
    create_signing_client, create_signing_client_with_nym_api, ClientArgs,
};
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod rewards;

pub(crate) async fn execute(
    global_args: ClientArgs,
    delegators: nym_cli_commands::validator::mixnet::delegators::MixnetDelegators,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match delegators.command {
        nym_cli_commands::validator::mixnet::delegators::MixnetDelegatorsCommands::Rewards(rewards) => {
            rewards::execute(global_args, rewards, network_details).await?
        }
        nym_cli_commands::validator::mixnet::delegators::MixnetDelegatorsCommands::Delegate(args) => {
            nym_cli_commands::validator::mixnet::delegators::delegate_to_mixnode::delegate_to_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::delegators::MixnetDelegatorsCommands::Undelegate(args) => {
            nym_cli_commands::validator::mixnet::delegators::undelegate_from_mixnode::undelegate_from_mixnode(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::delegators::MixnetDelegatorsCommands::List(args) => {
            nym_cli_commands::validator::mixnet::delegators::query_for_delegations::execute(args, create_signing_client_with_nym_api(global_args, network_details)?).await
        }
        _ => unreachable!(),
    }
    Ok(())
}
