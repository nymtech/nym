// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    vesting: nym_cli_commands::validator::vesting::VestingSchedule,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match vesting.command {
        Some(nym_cli_commands::validator::vesting::VestingScheduleCommands::Create(args)) => {
            nym_cli_commands::validator::vesting::create_vesting_schedule::create(
                args,
                create_signing_client(global_args, network_details)?,
                network_details,
            )
            .await
        }
        Some(nym_cli_commands::validator::vesting::VestingScheduleCommands::Query(args)) => {
            let address_from_args = args.address.clone();
            nym_cli_commands::validator::vesting::query_vesting_schedule::query(
                args,
                create_query_client(network_details)?,
                address_from_args,
            )
            .await
        }
        Some(nym_cli_commands::validator::vesting::VestingScheduleCommands::VestedBalance(
            args,
        )) => {
            let address_from_args = args.address.clone();
            nym_cli_commands::validator::vesting::balance::balance(
                args,
                create_query_client(network_details)?,
                address_from_args,
            )
            .await
        }
        Some(nym_cli_commands::validator::vesting::VestingScheduleCommands::WithdrawVested(
            args,
        )) => {
            nym_cli_commands::validator::vesting::withdraw_vested::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        _ => unreachable!(),
    }
    Ok(())
}
