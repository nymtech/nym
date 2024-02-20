// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::AccountId;

pub(crate) async fn execute(
    global_args: ClientArgs,
    account: nym_cli_commands::validator::account::Account,
    network_details: &NymNetworkDetails,
    mnemonic: Option<bip39::Mnemonic>,
) -> anyhow::Result<()> {
    match account.command {
        Some(nym_cli_commands::validator::account::AccountCommands::Create(args)) => {
            nym_cli_commands::validator::account::create::create_account(
                args,
                &network_details.chain_details.bech32_account_prefix,
            )
        }
        Some(nym_cli_commands::validator::account::AccountCommands::Balance(args)) => {
            let address_from_args = args.address.clone();
            nym_cli_commands::validator::account::balance::query_balance(
                args,
                &create_query_client(network_details)?,
                get_account_from_mnemonic_as_option(
                    global_args,
                    network_details,
                    address_from_args,
                ),
            )
            .await
        }
        Some(nym_cli_commands::validator::account::AccountCommands::PubKey(args)) => {
            let address_from_args = args.address.clone();
            nym_cli_commands::validator::account::pubkey::get_pubkey(
                args,
                &create_query_client(network_details)?,
                mnemonic,
                get_account_from_mnemonic_as_option(
                    global_args,
                    network_details,
                    address_from_args,
                ),
            )
            .await;
        }
        Some(nym_cli_commands::validator::account::AccountCommands::Send(args)) => {
            nym_cli_commands::validator::account::send::send(
                args,
                &create_signing_client(global_args, network_details)?,
            )
            .await;
        }
        Some(nym_cli_commands::validator::account::AccountCommands::SendMultiple(args)) => {
            nym_cli_commands::validator::account::send_multiple::send_multiple(
                args,
                &create_signing_client(global_args, network_details)?,
            )
            .await;
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn get_account_from_mnemonic(
    global_args: ClientArgs,
    network_details: &NymNetworkDetails,
    address: Option<AccountId>,
) -> anyhow::Result<Option<AccountId>> {
    Ok(address.or(Some(
        create_signing_client(global_args, network_details)?.address(),
    )))
}

fn get_account_from_mnemonic_as_option(
    global_args: ClientArgs,
    network_details: &NymNetworkDetails,
    address: Option<AccountId>,
) -> Option<AccountId> {
    get_account_from_mnemonic(global_args, network_details, address).unwrap_or(None)
}
