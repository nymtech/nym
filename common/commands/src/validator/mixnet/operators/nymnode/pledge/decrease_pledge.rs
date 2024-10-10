// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_mixnet_contract_common::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub decrease_by: u128,
}

pub async fn decrease_pledge(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!("Starting to decrease pledge");

    let coin = Coin::new(args.decrease_by, denom);

    let res = client
        .pledge_more(coin.into(), None)
        .await
        .expect("failed to decrease pledge!");

    info!("decreasing pledge: {:?}", res);
}
