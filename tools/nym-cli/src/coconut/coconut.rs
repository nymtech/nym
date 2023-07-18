// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_signing_client_with_nym_api, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    coconut: nym_cli_commands::coconut::Coconut,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match coconut.command {
        nym_cli_commands::coconut::CoconutCommands::IssueCredentials(args) => {
            nym_cli_commands::coconut::issue_credentials::execute(
                args,
                create_signing_client_with_nym_api(global_args, network_details)?,
            )
            .await
        }
    }
    Ok(())
}
