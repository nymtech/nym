// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use cosmwasm_std::Coin;
use log::{debug, info};

use nym_ecash_contract_common::msg::InstantiateMsg;
use nym_validator_client::nyxd::AccountId;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub group_addr: Option<AccountId>,

    #[clap(long)]
    pub multisig_addr: Option<AccountId>,

    #[clap(long)]
    pub holding_account: AccountId,

    #[clap(long, default_value = "75000000unym")]
    pub deposit_amount: Coin,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let group_addr = args.group_addr.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::GROUP_CONTRACT_ADDRESS)
            .expect("Multisig address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting multisig address to AccountId")
    });

    let multisig_addr = args.multisig_addr.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::MULTISIG_CONTRACT_ADDRESS)
            .expect("Multisig address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting multisig address to AccountId")
    });

    let instantiate_msg = InstantiateMsg {
        holding_account: args.holding_account.to_string(),
        group_addr: group_addr.to_string(),
        multisig_addr: multisig_addr.to_string(),
        deposit_amount: args.deposit_amount,
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
