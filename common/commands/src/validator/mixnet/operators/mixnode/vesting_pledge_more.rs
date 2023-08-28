// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;
use nym_mixnet_contract_common::Coin;
use nym_validator_client::nyxd::contract_traits::VestingSigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub amount: u128,
}

pub async fn vesting_pledge_more(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!("Starting vesting pledge more");

    let coin = Coin::new(args.amount, denom);

    let res = client
        .vesting_pledge_more(coin.into(), None)
        .await
        .expect("failed to pledge more!");

    info!("vesting pledge more: {:?}", res);
}
