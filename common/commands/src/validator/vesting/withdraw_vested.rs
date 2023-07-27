// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;

use nym_validator_client::nyxd::{
    traits::{VestingQueryClient, VestingSigningClient},
    Coin,
};

use crate::context::SigningClient;
use crate::utils::show_error;
use crate::utils::{pretty_coin, pretty_cosmwasm_coin};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Amount to transfer in micro denomination (e.g. unym or unyx)")]
    pub amount: u128,
}

pub async fn execute(args: Args, client: SigningClient) {
    let account_id = client.address();
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
                .get_balance(account_id, denom.to_string())
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| Coin::new(0u128, denom));

            println!(
                "Account {} has\n{} vested with {} available to be withdrawn to the main account (balance {})",
                &account_id,
                pretty_cosmwasm_coin(&res.amount),
                pretty_coin(&spendable_coins),
                pretty_coin(&liquid_account_balance),
            );
            println!();

            // execute withdraw

            let amount = Coin {
                amount: args.amount,
                denom: denom.to_string(),
            };

            info!(
                "Withdrawing {} ({}) from {}...",
                pretty_coin(&amount),
                &amount,
                &account_id
            );

            match client.withdraw_vested_coins(amount, None).await {
                Ok(res) => {
                    println!();
                    println!("SUCCESS ✅");
                    println!(
                        "Nodesguru: https://nym.explorers.guru/transaction/{}",
                        &res.transaction_hash
                    );
                    println!(
                        "Mintscan: https://www.mintscan.io/nyx/txs/{}",
                        &res.transaction_hash
                    );
                    println!("Transaction hash: {}", &res.transaction_hash);
                    println!("Gas used: {}", &res.gas_info.gas_used);
                    println!();
                }
                Err(e) => show_error(e),
            }

            // query for balances again
            let res = client
                .original_vesting(&vesting_address)
                .await
                .expect("vesting account does not exist");
            let spendable_coins = client
                .spendable_coins(&vesting_address, None)
                .await
                .unwrap_or_else(|_| Coin::new(0u128, denom));

            let liquid_account_balance = client
                .get_balance(account_id, denom.to_string())
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| Coin::new(0u128, denom));

            println!(
                "After withdrawal, account {} has\n{} vested with {} available to be withdrawn to the main account (balance {})",
                &account_id,
                pretty_cosmwasm_coin(&res.amount),
                pretty_coin(&spendable_coins),
                pretty_coin(&liquid_account_balance),
            );
        }
        Err(e) => show_error(e),
    }
}
