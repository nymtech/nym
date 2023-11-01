// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use cosmrs::{AccountId, Coin as CosmosCoin};
use log::info;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::cosmwasm_client::types::{ContractCodeId, InstantiateOptions};
use nym_validator_client::nyxd::Coin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    pub code_id: ContractCodeId,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(long)]
    pub label: Option<String>,

    #[clap(long)]
    pub init_message: String,

    #[clap(long)]
    pub admin: Option<AccountId>,

    #[clap(
        long,
        requires = "funds_denom",
        help = "Amount to supply as funds in micro denomination (e.g. unym or unyx)"
    )]
    pub funds: Option<u128>,

    #[clap(long, requires = "funds", help = "Set the denomination for the funds")]
    pub funds_denom: Option<String>,
}

pub async fn init(args: Args, client: SigningClient, network_details: &NymNetworkDetails) {
    info!("Starting contract instantiation!");

    let memo = args
        .memo
        .unwrap_or_else(|| "contract instantiation".to_owned());
    let label = args
        .label
        .unwrap_or_else(|| "Nym mixnet smart contract".to_owned());

    let funds: Vec<CosmosCoin> = match args.funds {
        Some(funds) => vec![Coin::new(
            funds,
            args.funds_denom
                .unwrap_or_else(|| network_details.chain_details.mix_denom.base.to_string()),
        )
        .into()],
        None => vec![],
    };

    // by default we make ourselves an admin, let me know if you don't like that behaviour
    let opts = Some(InstantiateOptions {
        funds,
        admin: Some(args.admin.unwrap_or_else(|| client.address())),
    });

    let msg: serde_json::Value =
        serde_json::from_str(&args.init_message).expect("failed to parse init message");

    // the EmptyMsg{} argument is equivalent to `--init-message='{}'`
    let res = client
        .instantiate(args.code_id, &msg, label, memo, opts, None)
        .await
        .expect("failed to instantiate the contract!");

    info!("Init result: {:?}", res);

    println!("{}", res.contract_address)
}
