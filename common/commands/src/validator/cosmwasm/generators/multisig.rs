// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use cosmwasm_std::Decimal;
use cw_utils::{Duration, Threshold};
use log::{debug, info};
use nym_multisig_contract_common::msg::InstantiateMsg;
use nym_validator_client::nyxd::AccountId;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub group_addr: String,

    #[clap(long, default_value_t = 67)]
    pub threshold: u128,

    #[clap(long, default_value_t = 3600)]
    pub max_voting_period: u64,

    #[clap(long)]
    pub ecash_contract_address: Option<AccountId>,

    #[clap(long)]
    pub coconut_dkg_contract_address: Option<AccountId>,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let ecash_contract_address = args.ecash_contract_address.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::ECASH_CONTRACT_ADDRESS)
            .expect("Coconut bandwidth contract address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting bandwidth contract address to AccountId")
    });

    let coconut_dkg_contract_address = args.coconut_dkg_contract_address.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::COCONUT_DKG_CONTRACT_ADDRESS)
            .expect("Coconut DKG contract address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting DKG contract address to AccountId")
    });

    let instantiate_msg = InstantiateMsg {
        group_addr: args.group_addr,
        threshold: Threshold::AbsolutePercentage {
            percentage: Decimal::from_atomics(args.threshold, 2)
                .expect("threshold can't be converted to Decimal"),
        },
        max_voting_period: Duration::Time(args.max_voting_period),
        executor: None,
        proposal_deposit: None,
        coconut_bandwidth_contract_address: ecash_contract_address.to_string(),
        coconut_dkg_contract_address: coconut_dkg_contract_address.to_string(),
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
