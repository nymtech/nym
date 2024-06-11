// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::create_query_client_with_nym_api;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    query: nym_cli_commands::validator::mixnet::query::MixnetQuery,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match query.command {
        nym_cli_commands::validator::mixnet::query::MixnetQueryCommands::Mixnodes(args) => {
            nym_cli_commands::validator::mixnet::query::query_all_mixnodes::query(
                args,
                &create_query_client_with_nym_api(network_details)?,
            )
            .await
        }
        nym_cli_commands::validator::mixnet::query::MixnetQueryCommands::Gateways(args) => {
            nym_cli_commands::validator::mixnet::query::query_all_gateways::query(
                args,
                &create_query_client_with_nym_api(network_details)?,
            )
            .await
        }
    }
    Ok(())
}
