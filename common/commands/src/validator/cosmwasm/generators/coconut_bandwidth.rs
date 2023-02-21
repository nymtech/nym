// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use log::{debug, info};

use coconut_bandwidth_contract_common::msg::InstantiateMsg;
use validator_client::nyxd::AccountId;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub pool_addr: String,

    #[clap(long)]
    pub multisig_addr: Option<AccountId>,

    #[clap(long)]
    pub mix_denom: Option<String>,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let multisig_addr = args.multisig_addr.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::REWARDING_VALIDATOR_ADDRESS)
            .expect("Multisig address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting multisig address to AccountId")
    });

    let mix_denom = args.mix_denom.unwrap_or_else(|| {
        std::env::var(nym_network_defaults::var_names::MIX_DENOM).expect("Mix denom has to be set")
    });

    let instantiate_msg = InstantiateMsg {
        pool_addr: args.pool_addr,
        multisig_addr: multisig_addr.to_string(),
        mix_denom,
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
