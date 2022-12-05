// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use cosmrs::AccountId;
use cosmwasm_std::Coin as CosmWasmCoin;
use log::{error, info};

use validator_client::nymd::{Coin, VestingQueryClient};

use crate::context::QueryClient;
use crate::utils::show_error;
use crate::utils::{pretty_coin, pretty_cosmwasm_coin};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the account address to get the balance for")]
    pub address: Option<AccountId>,
}

pub async fn query(args: Args, client: QueryClient, address_from_mnemonic: Option<AccountId>) {
    if args.address.is_none() && address_from_mnemonic.is_none() {
        error!("Please specify an account address or a mnemonic to get the balance for");
        return;
    }

    let account_id = args
        .address
        .unwrap_or_else(|| address_from_mnemonic.expect("please provide a mnemonic"));

    info!("Checking account {} for a vesting schedule...", account_id);

    let vesting_address = account_id.to_string();
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!(
        "Getting vesting schedule information for {}...",
        &vesting_address
    );

    let liquid_account_balance = client
        .get_balance(&account_id, denom.to_string())
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| Coin::new(0u128, denom));
    let original_vesting = client.original_vesting(&vesting_address).await;
    let start_time = client.vesting_start_time(&vesting_address).await;
    let end_time = client.vesting_end_time(&vesting_address).await;
    let vested_coins = client.vested_coins(&vesting_address, None).await;
    let spendable_coins = client.spendable_coins(&vesting_address, None).await;
    let locked_coins = client.locked_coins(&vesting_address, None).await;

    // TODO: get better copy text for what these are
    let vesting_coins = client.vesting_coins(&vesting_address, None).await;
    let delegated_vesting = client.delegated_vesting(&vesting_address, None).await;
    let delegated_free = client.delegated_free(&vesting_address, None).await;

    original_vesting.as_ref().map_or_else(show_error, |res| {
        println!(
            "Amount:            {}   ({})",
            pretty_cosmwasm_coin(&res.amount),
            res.amount
        );
        println!("No of periods:     {}", res.number_of_periods);
        println!(
            "Duration each:     {}",
            time::Duration::seconds(res.period_duration as i64)
        );
    });

    start_time.as_ref().map_or_else(show_error, |res| {
        println!(
            "Start date:        {}",
            time::OffsetDateTime::from_unix_timestamp(res.seconds() as i64)
                .expect("unable to parse vesting start timestamp")
                .date()
        );
    });

    end_time.map_or_else(show_error, |res| {
        println!(
            "End date:          {}",
            time::OffsetDateTime::from_unix_timestamp(res.seconds() as i64)
                .expect("unable to parse vesting end timestamp")
                .date()
        );
    });

    vested_coins.map_or_else(show_error, |res| {
        println!("Vested balance:    {}   ({})", pretty_coin(&res), res);
    });

    if let Ok(res) = original_vesting {
        if let Ok(start) = start_time {
            let amount_in_each_period = res.amount.amount.u128() / res.number_of_periods as u128;
            let coin_in_each_period = CosmWasmCoin::new(amount_in_each_period, denom);
            println!();
            println!("Vesting schedule:");
            for period in 1..(res.number_of_periods as u64 + 1) {
                let date = time::OffsetDateTime::from_unix_timestamp(
                    (start.seconds() + period * res.period_duration) as i64,
                )
                .expect("unable to parse vesting start timestamp")
                .date();
                let amount_in_vested =
                    period as u128 * res.amount.amount.u128() / res.number_of_periods as u128;
                let coin_in_vested = CosmWasmCoin::new(amount_in_vested, denom);
                println!(
                    "{}.  {}    {}  => {}",
                    period,
                    date,
                    pretty_cosmwasm_coin(&coin_in_each_period),
                    pretty_cosmwasm_coin(&coin_in_vested),
                );
            }
        }
    }

    spendable_coins.map_or_else(show_error, |res| {
        println!();
        println!("This account has the following vested tokens available either to be withdrawn to the main account, or to be delegated:");
        println!("Spendable coins:   {}   ({})", pretty_coin(&res), res);
    });

    locked_coins.map_or_else(show_error, |res| {
        println!();
        if res.amount > 0 {
            println!("This account has delegated more than the current cap, so the following balance is unavailable for bonding or delegation:");
            println!("Locked balance:    {}   ({})", pretty_coin(&res), res);
        } else {
            println!("This account is not capped and can use the spendable balance for bonding or delegations:");
            println!("Locked balance:    {}   ({})", pretty_coin(&res), res);
        }
    });

    println!();
    println!("The following are shown for information (more help text will follow soon):");
    vesting_coins.map_or_else(show_error, |res| {
        println!("Vesting coins:     {}   ({})", pretty_coin(&res), res);
    });
    delegated_vesting.map_or_else(show_error, |res| {
        println!("Delegated vesting: {}   ({})", pretty_coin(&res), res);
    });
    delegated_free.map_or_else(show_error, |res| {
        println!("Delegation free:   {}   ({})", pretty_coin(&res), res);
    });

    println!();
    println!(
        "The main account {} also has a regular balance of:",
        &account_id
    );
    println!(
        "{}  ({})",
        pretty_coin(&liquid_account_balance),
        &liquid_account_balance
    );
}
