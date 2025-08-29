// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::ClientArgs;
use nym_cli_commands::internal::ecash::InternalEcashCommands;
use nym_network_defaults::NymNetworkDetails;

pub(super) async fn execute(
    global_args: ClientArgs,
    nym_network_details: &NymNetworkDetails,
    ecash: nym_cli_commands::internal::ecash::InternalEcash,
) -> anyhow::Result<()> {
    // I reckon those will be needed later
    let _ = global_args;
    let _ = nym_network_details;

    match ecash.command {
        InternalEcashCommands::GenerateWithdrawalRequest(args) => {
            nym_cli_commands::internal::ecash::withdrawal_request::generate_withdrawal_request(args)
                .await
        }
        InternalEcashCommands::GenerateKeypair(args) => {
            nym_cli_commands::internal::ecash::generate_keypair::generate_ecash_keypair(args)
        }
    }
}
