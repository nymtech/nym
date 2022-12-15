// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::{debug, info};

use cosmwasm_std::Decimal;
use cw_utils::{Duration, Threshold};
use multisig_contract_common::msg::InstantiateMsg;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long, empty_values = false)]
    pub group_addr: String,

    #[clap(long, default_value_t = 67)]
    pub threshold: u128,

    #[clap(long, default_value_t = 3600)]
    pub max_voting_period: u64,

    #[clap(long)]
    pub coconut_bandwidth_contract_address: Option<String>,

    #[clap(long)]
    pub coconut_dkg_contract_address: Option<String>,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let coconut_bandwidth_contract_address =
        args.coconut_bandwidth_contract_address.unwrap_or_else(|| {
            std::env::var(network_defaults::var_names::COCONUT_BANDWIDTH_CONTRACT_ADDRESS)
                .expect("Coconut bandwidth contract address has to be set")
        });

    let coconut_dkg_contract_address = args.coconut_dkg_contract_address.unwrap_or_else(|| {
        std::env::var(network_defaults::var_names::COCONUT_DKG_CONTRACT_ADDRESS)
            .expect("Coconut DKG contract address has to be set")
    });

    let instantiate_msg = InstantiateMsg {
        group_addr: args.group_addr,
        threshold: Threshold::AbsolutePercentage {
            percentage: Decimal::from_atomics(args.threshold, 2)
                .expect("threshold can't be converted to Decimal"),
        },
        max_voting_period: Duration::Time(args.max_voting_period),
        coconut_bandwidth_contract_address,
        coconut_dkg_contract_address,
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{}", res)
}
