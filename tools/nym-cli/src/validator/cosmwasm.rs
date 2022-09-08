// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::NymNetworkDetails;
use nym_cli_commands::context::{create_signing_client, ClientArgs};

pub(crate) async fn execute(
    global_args: ClientArgs,
    cosmwasm: nym_cli_commands::validator::cosmwasm::Cosmwasm,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match cosmwasm.command {
        Some(nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Upload(args)) => {
            nym_cli_commands::validator::cosmwasm::upload_contract::upload(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        Some(nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Init(args)) => {
            nym_cli_commands::validator::cosmwasm::init_contract::init(
                args,
                create_signing_client(global_args, network_details)?,
                network_details,
            )
            .await
        }
        Some(nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Migrate(args)) => {
            nym_cli_commands::validator::cosmwasm::migrate_contract::migrate(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        Some(nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Execute(args)) => {
            nym_cli_commands::validator::cosmwasm::execute_contract::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        _ => unreachable!(),
    }
    Ok(())
}
