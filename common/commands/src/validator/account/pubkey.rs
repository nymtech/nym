// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::{error, info};
use nym_validator_client::nyxd::{AccountId, CosmWasmClient};
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;

use crate::context::QueryClient;
use crate::utils::show_error;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(
        help = "Optionally, show the public key for this account address, otherwise generate the account address from the mnemonic"
    )]
    pub address: Option<AccountId>,

    #[clap(long)]
    #[clap(help = "If set, get the public key from the mnemonic, rather than querying for it")]
    pub from_mnemonic: bool,
}

pub async fn get_pubkey(
    args: Args,
    client: &QueryClient,
    mnemonic: Option<bip39::Mnemonic>,
    address_from_mnemonic: Option<AccountId>,
) {
    if args.address.is_none() && address_from_mnemonic.is_none() {
        error!("Please specify an account address or a mnemonic to get the balance for");
        return;
    }

    let address = args
        .address
        .unwrap_or_else(|| address_from_mnemonic.expect("please provide a mnemonic"));

    if args.from_mnemonic {
        let prefix = client
            .current_chain_details()
            .bech32_account_prefix
            .as_str();
        get_pubkey_from_mnemonic(address, prefix, mnemonic.expect("mnemonic not set"));
        return;
    }

    get_pubkey_from_chain(address, client).await;
}

pub fn get_pubkey_from_mnemonic(address: AccountId, prefix: &str, mnemonic: bip39::Mnemonic) {
    let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
    match wallet.try_derive_accounts() {
        Ok(accounts) => match accounts.iter().find(|a| *a.address() == address) {
            Some(account) => {
                println!("{}", account.public_key().to_string());
            }
            None => {
                error!("Could not derive key that matches {}", address)
            }
        },
        Err(e) => {
            error!("Failed to derive accounts. {}", e);
        }
    }
}

pub async fn get_pubkey_from_chain(address: AccountId, client: &QueryClient) {
    info!("Getting public key for address {} from chain...", address);
    match client.get_account(&address).await {
        Ok(Some(account)) => {
            if let Ok(base_account) = account.try_get_base_account() {
                if let Some(pubkey) = base_account.pubkey {
                    println!("{}", pubkey.to_string());
                } else {
                    println!("No account associated with address {address}");
                }
            }
        }
        Ok(None) => {
            println!("No account associated with address {address}");
        }
        Err(e) => show_error(e),
    }
}
