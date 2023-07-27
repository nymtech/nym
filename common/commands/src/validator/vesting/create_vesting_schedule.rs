// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use log::info;

use nym_mixnet_contract_common::Coin;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::traits::VestingSigningClient;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::{CosmosCoin, Denom};
use nym_vesting_contract_common::messages::VestingSpecification;
use nym_vesting_contract_common::PledgeCap;

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

    #[clap(
        long,
        help = "Pledge cap as either absolute uNYM value or percentage, floats need to be in the 0.0 to 1.0 range and will be parsed as percentages, integers will be parsed as uNYM"
    )]
    pub pledge_cap: Option<PledgeCap>,
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
            args.pledge_cap,
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
