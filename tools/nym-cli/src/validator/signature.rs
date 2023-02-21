// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::create_query_client;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    signature: nym_cli_commands::validator::signature::Signature,
    network_details: &NymNetworkDetails,
    mnemonic: Option<bip39::Mnemonic>,
) -> anyhow::Result<()> {
    match signature.command {
        Some(nym_cli_commands::validator::signature::SignatureCommands::Sign(args)) => {
            nym_cli_commands::validator::signature::sign::sign(
                args,
                &network_details.chain_details.bech32_account_prefix,
                mnemonic,
            )
        }
        Some(nym_cli_commands::validator::signature::SignatureCommands::Verify(args)) => {
            nym_cli_commands::validator::signature::verify::verify(
                args,
                &create_query_client(network_details)?,
            )
            .await
        }
        _ => unreachable!(),
    }
    Ok(())
}
