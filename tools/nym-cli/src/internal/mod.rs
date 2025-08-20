// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::ClientArgs;
use nym_cli_commands::internal::InternalCommands;
use nym_network_defaults::NymNetworkDetails;

mod ecash;
mod nyx;

pub(super) async fn execute(
    global_args: ClientArgs,
    internal: nym_cli_commands::internal::Internal,
    nym_network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match internal.command {
        InternalCommands::Ecash(ecash_commands) => {
            ecash::execute(global_args, nym_network_details, ecash_commands).await
        }
        InternalCommands::Nyx(nyx_commands) => {
            nyx::execute(global_args, nym_network_details, nyx_commands).await
        }
    }
}
