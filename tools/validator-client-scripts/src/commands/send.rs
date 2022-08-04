// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use crate::Client;
use clap::Parser;
use log::{info, warn};
use validator_client::nymd::{AccountId, CosmosCoin, Denom};

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub recipient: AccountId,

    #[clap(
        long,
        help = "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub amount: u64,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(long)]
    pub gas: Option<u64>,

    #[clap(short, long)]
    pub force: bool,
}

pub(crate) async fn send(client: Client, args: Args, denom: &str) {
    info!("Starting token sending!");

    let memo = args.memo.unwrap_or_else(|| "Sending tokens".to_owned());

    // if we're trying to bond less than 1 token
    if args.amount < 1_000_000 && !args.force {
        warn!("You're trying to send only {}{} which is less than 1 full token. Are you sure that's what you want? If so, run with `--force` or `-f` flag", args.amount, denom);
        return;
    }

    let coin = CosmosCoin {
        denom: Denom::from_str(denom).unwrap(),
        amount: args.amount.into(),
    };

    let res = client
        .send(&args.recipient, vec![coin.into()], memo, None)
        .await
        .expect("failed to send tokens!");

    info!("Sending result: {:?}", res)
}
