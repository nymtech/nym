// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

#[allow(dead_code)]
pub(crate) async fn execute(
    global_args: ClientArgs,
    settings: nym_cli_commands::validator::mixnet::operators::gateway::settings::MixnetOperatorsGatewaySettings,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match settings.command {
        nym_cli_commands::validator::mixnet::operators::gateway::settings::MixnetOperatorsGatewaySettingsCommands::UpdateConfig(args) => {
            nym_cli_commands::validator::mixnet::operators::gateway::settings::update_config::update_config(args, create_signing_client(global_args, network_details)?).await
        }
        nym_cli_commands::validator::mixnet::operators::gateway::settings::MixnetOperatorsGatewaySettingsCommands::VestingUpdateConfig(args) => {
            nym_cli_commands::validator::mixnet::operators::gateway::settings::vesting_update_config::vesting_update_config(args, create_signing_client(global_args, network_details)?).await
        }
    }
    Ok(())
}
