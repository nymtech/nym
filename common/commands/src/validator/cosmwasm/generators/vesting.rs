// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use log::{debug, info};

use nym_validator_client::nyxd::AccountId;
use nym_vesting_contract_common::InitMsg;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub mixnet_contract_address: Option<AccountId>,

    #[clap(long)]
    pub mix_denom: Option<String>,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {args:?}");

    let mixnet_contract_address = args.mixnet_contract_address.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::MIXNET_CONTRACT_ADDRESS)
            .expect("Mixnet contract address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting mixnet address to AccountId")
    });

    let mix_denom = args.mix_denom.unwrap_or_else(|| {
        std::env::var(nym_network_defaults::var_names::MIX_DENOM).expect("Mix denom has to be set")
    });

    let instantiate_msg = InitMsg {
        mixnet_contract_address: mixnet_contract_address.to_string(),
        mix_denom,
    };

    debug!("instantiate_msg: {instantiate_msg:?}");

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
