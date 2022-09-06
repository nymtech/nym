// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::NymNetworkDetails;
use nym_cli_commands::context::{create_signing_client, ClientArgs};

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
            nym_cli_commands::validator::vesting::query_vesting_schedule::query(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        Some(nym_cli_commands::validator::vesting::VestingScheduleCommands::VestedBalance(
            args,
        )) => {
            nym_cli_commands::validator::vesting::balance::balance(
                args,
                create_signing_client(global_args, network_details)?,
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
