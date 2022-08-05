// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use validator_client::nymd::cosmwasm_client::types::{ContractCodeId, EmptyMsg};

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub code_id: ContractCodeId,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(long)]
    pub init_message: Option<String>,

    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn migrate(client: Client, args: Args) {
    println!("Starting contract migration!");

    let memo = args.memo.unwrap_or_else(|| "contract migration".to_owned());
    let contract_address = client.mixnet_contract_address();

    // the EmptyMsg{} argument is equivalent to `--init-message='{}'`
    let res = if let Some(raw_msg) = args.init_message {
        let msg: serde_json::Value =
            serde_json::from_str(&raw_msg).expect("failed to parse init message");

        client
            .migrate(contract_address, args.code_id, &msg, memo, None)
            .await
            .expect("failed to instantiate the contract!")
    } else {
        client
            .migrate(contract_address, args.code_id, &EmptyMsg {}, memo, None)
            .await
            .expect("failed to instantiate the contract!")
    };

    info!("Migrate result: {:?}", res);
}
