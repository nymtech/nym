// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use log::info;

use mixnet_contract_common::Coin;
use network_defaults::NymNetworkDetails;
use validator_client::nymd::AccountId;
use validator_client::nymd::VestingSigningClient;
use validator_client::nymd::{CosmosCoin, Denom};
use vesting_contract_common::messages::VestingSpecification;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub periods_seconds: Option<u64>,

    #[clap(long)]
    pub number_of_periods: Option<u64>,

    #[clap(long)]
    pub start_time: Option<u64>,

    #[clap(long)]
    pub address: String,

    #[clap(long)]
    pub amount: u64,

    #[clap(long)]
    pub staking_address: Option<String>,
}

pub async fn create(args: Args, client: SigningClient, network_details: &NymNetworkDetails) {
    info!("Creating vesting schedule!");

    let vesting = VestingSpecification::new(
        args.start_time,
        args.periods_seconds,
        args.number_of_periods,
    );

    let denom = network_details.chain_details.mix_denom.base.to_string();

    let coin = Coin::new(args.amount.into(), &denom);

    let res = client
        .create_periodic_vesting_account(
            &args.address,
            args.staking_address,
            Some(vesting),
            coin.into(),
            None,
        )
        .await
        .expect("creating vesting schedule for the user!");

    //send 1 coin
    let coin_amount: u64 = 1_000_000;

    let coin = CosmosCoin {
        denom: Denom::from_str(&denom).unwrap(),
        amount: coin_amount.into(),
    };

    let send_coin_response = client
        .send(
            &AccountId::from_str(&args.address).unwrap(),
            vec![coin.into()],
            "payment made :)",
            None,
        )
        .await
        .unwrap();

    info!("Vesting result: {:?}", res);
    info!("Coin send result: {:?}", send_coin_response);
}
