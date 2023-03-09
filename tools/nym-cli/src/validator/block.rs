// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::create_query_client;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    block: nym_cli_commands::validator::block::Block,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match block.command {
        Some(nym_cli_commands::validator::block::BlockCommands::Get(args)) => {
            nym_cli_commands::validator::block::get::query_for_block(
                args,
                &create_query_client(network_details)?,
            )
            .await
        }
        Some(nym_cli_commands::validator::block::BlockCommands::Time(args)) => {
            nym_cli_commands::validator::block::block_time::query_for_block_time(
                args,
                &create_query_client(network_details)?,
            )
            .await
        }
        Some(nym_cli_commands::validator::block::BlockCommands::CurrentHeight(_args)) => {
            nym_cli_commands::validator::block::current_height::query_current_block_height(
                &create_query_client(network_details)?,
            )
            .await
        }
        _ => unreachable!(),
    }
    Ok(())
}
