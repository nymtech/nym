// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::NymNetworkDetails;
use nym_cli_commands::context::{create_signing_client, ClientArgs};

pub(crate) async fn execute(
    global_args: ClientArgs,
    settings: nym_cli_commands::validator::mixnet::operators::mixnode::settings::MixnetOperatorsMixnodeSettings,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match settings.command {
        nym_cli_commands::validator::mixnet::operators::mixnode::settings::MixnetOperatorsMixnodeSettingsCommands::UpdateConfig(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::settings::update_config::update_config(args, create_signing_client(global_args, network_details)?).await
        }
        _ => unreachable!(),
    }
    Ok(())
}
