// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::{error, info};

use nym_validator_client::nyxd::{AccountId, CosmWasmClient};

use crate::context::QueryClient;
use crate::utils::{pretty_coin, show_error};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The account address to get the balance for")]
    pub address: Option<AccountId>,

    #[clap(long)]
    #[clap(help = "Optional currency to show balance for")]
    pub denom: Option<String>,

    #[clap(long, requires = "denom")]
    #[clap(help = "Optionally hide the denom")]
    pub hide_denom: bool,

    #[clap(long)]
    #[clap(help = "Show as a raw value")]
    pub raw: bool,
}

pub async fn query_balance(
    args: Args,
    client: &QueryClient,
    address_from_mnemonic: Option<AccountId>,
) {
    if args.address.is_none() && address_from_mnemonic.is_none() {
        error!("Please specify an account address or a mnemonic to get the balance for");
        return;
    }

    let address = args
        .address
        .unwrap_or_else(|| address_from_mnemonic.expect("please provide a mnemonic"));

    info!("Getting balance for {}...", address);

    match client.get_all_balances(&address).await {
        Ok(coins) => {
            if coins.is_empty() {
                println!("No balance");
                return;
            }

            let denom = args.denom.unwrap_or_default();

            for coin in coins {
                if denom.is_empty() || denom.eq_ignore_ascii_case(&coin.denom) {
                    if args.raw {
                        if !args.hide_denom {
                            println!("{coin}");
                        } else {
                            println!("{}", coin.amount);
                        }
                    } else {
                        println!("{}", pretty_coin(&coin));
                    }
                }
            }
        }
        Err(e) => show_error(e),
    }
}
