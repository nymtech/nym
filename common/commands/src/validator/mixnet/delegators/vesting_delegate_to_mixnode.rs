// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;

use mixnet_contract_common::Coin;
use validator_client::nymd::VestingSigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub identity_key: String,

    #[clap(long)]
    pub amount: u128,
}

pub async fn vesting_delegate_to_mixnode(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!("Starting vesting delegation to mixnode");

    let coin = Coin::new(args.amount, denom);

    let res = client
        .vesting_delegate_to_mixnode(&*args.identity_key, coin.into(), None)
        .await
        .expect("failed to delegate to mixnode!");

    info!("vesting delegating to mixnode: {:?}", res);
}
