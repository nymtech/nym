// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;
use serde_json::json;

use nym_validator_client::nyxd::{AccountId, Coin};

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser, help = "The recipient account address")]
    pub recipient: AccountId,

    #[clap(
        value_parser,
        help = "Amount to transfer in micro denomination (e.g. unym or unyx)"
    )]
    pub amount: u128,

    #[clap(long, help = "Override the denomination")]
    pub denom: Option<String>,

    #[clap(long)]
    pub memo: Option<String>,
}

pub async fn send(args: Args, client: &SigningClient) {
    let memo = args
        .memo
        .unwrap_or_else(|| "Sending tokens with nym-cli".to_owned());
    let denom = args
        .denom
        .unwrap_or_else(|| client.current_chain_details().mix_denom.base.clone());

    let coin = Coin {
        denom,
        amount: args.amount,
    };

    info!(
        "Sending {} {} from {} to {}...",
        coin.amount,
        coin.denom,
        client.address(),
        args.recipient
    );

    let res = client
        .send(&args.recipient, vec![coin], memo, None)
        .await
        .expect("failed to send tokens!");

    info!("Sending result: {}", json!(res));

    println!();
    println!(
        "Nodesguru: https://nym.explorers.guru/transaction/{}",
        &res.hash
    );
    println!("Mintscan: https://ping.pub/nyx/tx/{}", &res.hash);
    println!("Transaction result code: {}", &res.tx_result.code.value());
    println!("Transaction hash: {}", &res.hash);
}
