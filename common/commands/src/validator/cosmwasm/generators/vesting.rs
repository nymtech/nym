// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::{debug, info};

use vesting_contract_common::InitMsg;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long, empty_values = false)]
    pub mixnet_contract_address: String,

    #[clap(long, default_value = "unymt", empty_values = false)]
    pub mix_denom: String,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let instantiate_msg = InitMsg {
        mixnet_contract_address: args.mixnet_contract_address,
        mix_denom: args.mix_denom,
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{}", res)
}
