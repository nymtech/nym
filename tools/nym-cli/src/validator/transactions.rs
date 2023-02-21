// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::create_query_client;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    transactions: nym_cli_commands::validator::transactions::Transactions,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match transactions.command {
        Some(nym_cli_commands::validator::transactions::TransactionsCommands::Get(args)) => {
            nym_cli_commands::validator::transactions::get_transaction::get(
                args,
                &create_query_client(network_details)?,
            )
            .await
        }
        Some(nym_cli_commands::validator::transactions::TransactionsCommands::Query(args)) => {
            nym_cli_commands::validator::transactions::query_transactions::query(
                args,
                &create_query_client(network_details)?,
            )
            .await
        }
        _ => unreachable!(),
    }
    Ok(())
}
