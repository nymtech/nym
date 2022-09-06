// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::NymNetworkDetails;
use nym_cli_commands::context::{create_signing_client, ClientArgs};

pub(crate) async fn execute(
    global_args: ClientArgs,
    rewards: nym_cli_commands::validator::mixnet::delegators::rewards::MixnetDelegatorsReward,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match rewards.command {
        nym_cli_commands::validator::mixnet::delegators::rewards::MixnetDelegatorsRewardCommands::Claim(args) => {
            nym_cli_commands::validator::mixnet::delegators::rewards::claim_delegator_reward::claim_delegator_reward(args, create_signing_client(global_args, network_details)?).await
        }
        _ => unreachable!(),
    }
    Ok(())
}
