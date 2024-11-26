// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::nymnode::settings::MixnetOperatorsNymNodeSettingsCommands;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    settings: nym_cli_commands::validator::mixnet::operators::nymnode::settings::MixnetOperatorsNymNodeSettings,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match settings.command {
        MixnetOperatorsNymNodeSettingsCommands::UpdateConfig(args) => {
            nym_cli_commands::validator::mixnet::operators::nymnode::settings::update_config::update_config(args, create_signing_client(global_args, network_details)?).await
        },
        MixnetOperatorsNymNodeSettingsCommands::UpdateCostParameters(args) => {
            nym_cli_commands::validator::mixnet::operators::nymnode::settings::update_cost_params::update_cost_params(args, create_signing_client(global_args, network_details)?).await?
        }
    }
    Ok(())
}
