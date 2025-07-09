// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::show_error_passthrough;
use clap::Parser;
use cosmrs::AccountId;
use log::info;
use nym_validator_client::nyxd::cosmwasm_client::types::{ContractCodeId, EmptyMsg};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    pub contract_address: AccountId,

    #[clap(long)]
    pub code_id: ContractCodeId,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(long)]
    pub init_message: Option<String>,
}

pub async fn migrate(args: Args, client: SigningClient) {
    println!("Starting contract migration!");

    let memo = args.memo.unwrap_or_else(|| "contract migration".to_owned());
    let contract_address = args.contract_address;

    // the EmptyMsg{} argument is equivalent to `--init-message='{}'`
    let res = if let Some(raw_msg) = args.init_message {
        let msg: serde_json::Value =
            serde_json::from_str(&raw_msg).expect("failed to parse init message");

        client
            .migrate(&contract_address, args.code_id, &msg, memo, None)
            .await
            .map_err(show_error_passthrough)
            .expect("failed to migrate the contract!")
    } else {
        client
            .migrate(&contract_address, args.code_id, &EmptyMsg {}, memo, None)
            .await
            .map_err(show_error_passthrough)
            .expect("failed to migrate the contract!")
    };

    info!("Migrate result: {res:?}");
}
