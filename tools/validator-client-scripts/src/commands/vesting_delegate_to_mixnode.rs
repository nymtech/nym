// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use mixnet_contract_common::Coin;
use validator_client::nymd::VestingSigningClient;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub gas: Option<u64>,

    #[clap(long)]
    pub identity_key: String,

    #[clap(long)]
    pub amount: u128,
}

pub(crate) async fn vesting_delegate_to_mixnode(client: Client, args: Args, denom: &str) {
    info!("Starting vesting delegation to mixnode");

    let coin = Coin::new(args.amount, denom);

    let res = client
        .vesting_delegate_to_mixnode(&*args.identity_key, coin.into(), None)
        .await
        .expect("failed to delegate to mixnode!");

    info!("vesting delegating to mixnode: {:?}", res);
}
