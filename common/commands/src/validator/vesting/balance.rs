// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use cosmrs::AccountId;
use log::{error, info};

use nym_validator_client::nyxd::{traits::VestingQueryClient, Coin};

use crate::context::QueryClient;
use crate::utils::show_error;
use crate::utils::{pretty_coin, pretty_cosmwasm_coin};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the account address to get the balance for")]
    pub address: Option<AccountId>,
}

pub async fn balance(args: Args, client: QueryClient, address_from_mnemonic: Option<AccountId>) {
    if args.address.is_none() && address_from_mnemonic.is_none() {
        error!("Please specify an account address or a mnemonic to get the balance for");
        return;
    }

    let account_id = args
        .address
        .unwrap_or_else(|| address_from_mnemonic.expect("please provide a mnemonic"));

    let vesting_address = account_id.to_string();
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!(
        "Getting vesting schedule information for {}...",
        &vesting_address
    );

    let original_vesting = client.original_vesting(&vesting_address).await;

    match original_vesting {
        Ok(res) => {
            let spendable_coins = client
                .spendable_coins(&vesting_address, None)
                .await
                .unwrap_or_else(|_| Coin::new(0u128, denom));
            let liquid_account_balance = client
                .get_balance(&account_id, denom.to_string())
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| Coin::new(0u128, denom));

            println!(
                "Account {} has\n{} vested with\n{} available to be withdrawn to the main account (balance {})",
                &account_id,
                pretty_cosmwasm_coin(&res.amount),
                pretty_coin(&spendable_coins),
                pretty_coin(&liquid_account_balance),
            );
        }
        Err(e) => show_error(e),
    }
}
