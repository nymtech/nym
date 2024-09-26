// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::nymnode::pledge::MixnetOperatorsNymNodePledgeCommands;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    pledge: nym_cli_commands::validator::mixnet::operators::nymnode::pledge::MixnetOperatorsNymNodePledge,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match pledge.command {
        MixnetOperatorsNymNodePledgeCommands::Increase(args) => {
            nym_cli_commands::validator::mixnet::operators::nymnode::pledge::increase_pledge::increase_pledge(args, create_signing_client(global_args, network_details)?).await
        },
        MixnetOperatorsNymNodePledgeCommands::Decrease(args) => {
            nym_cli_commands::validator::mixnet::operators::nymnode::pledge::decrease_pledge::decrease_pledge(args, create_signing_client(global_args, network_details)?).await
        }
    }
    Ok(())
}
