// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_cli_commands::internal::nyx::InternalNyxCommands;
use nym_network_defaults::NymNetworkDetails;

pub(super) async fn execute(
    global_args: ClientArgs,
    nym_network_details: &NymNetworkDetails,
    nyx: nym_cli_commands::internal::nyx::InternalNyx,
) -> anyhow::Result<()> {
    match nyx.command {
        InternalNyxCommands::ForceAdvanceEpoch(args) => {
            nym_cli_commands::internal::nyx::force_advance_epoch::force_advance_epoch(
                args,
                create_signing_client(global_args, nym_network_details)?,
            )
            .await
        }
    }
}
