// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use cosmrs::AccountId;
use log::{error, info};
use nym_validator_client::nyxd::Coin;
use serde_json::{json, Value};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The address of contract to execute")]
    pub contract_address: AccountId,

    #[clap(value_parser)]
    #[clap(help = "JSON encoded method arguments")]
    pub json_args: String,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(
        value_parser,
        requires = "funds_denom",
        help = "Amount to supply as funds in micro denomination (e.g. unym or unyx)"
    )]
    pub funds: Option<u128>,

    #[clap(long, requires = "funds", help = "Set the denomination for the funds")]
    pub funds_denom: Option<String>,
}

pub async fn execute(args: Args, client: SigningClient) {
    info!("Starting contract method execution!");

    let json_args: Value =
        serde_json::from_str(&args.json_args).expect("Unable to parse JSON args");

    let memo = args
        .memo
        .unwrap_or_else(|| "nym-cli execute contract method".to_owned());

    let funds = match args.funds {
        Some(funds) => vec![Coin::new(
            funds,
            args.funds_denom.expect("denom for funds not set"),
        )],
        None => vec![],
    };

    match client
        .execute(&args.contract_address, &json_args, None, memo, funds)
        .await
    {
        Ok(res) => info!("SUCCESS ✅\n{}", json!(res)),
        Err(e) => error!("FAILURE ❌\n{e}"),
    }
}
