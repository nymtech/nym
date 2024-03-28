// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::nymnode::rewards::MixnetOperatorsNymNodeRewardsCommands;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    rewards: nym_cli_commands::validator::mixnet::operators::nymnode::rewards::MixnetOperatorsNymNodeRewards,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match rewards.command {
        MixnetOperatorsNymNodeRewardsCommands::Claim(args) => {
            nym_cli_commands::validator::mixnet::operators::nymnode::rewards::claim_operator_reward::claim_operator_reward(args, create_signing_client(global_args, network_details)?).await
        }
    }
    Ok(())
}
