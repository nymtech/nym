// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use mixnet_contract_common::InstantiateMsg;
use validator_client::nymd::cosmwasm_client::types::{ContractCodeId, InstantiateOptions};

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub code_id: ContractCodeId,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(long)]
    pub label: Option<String>,

    #[clap(long)]
    pub init_message: Option<String>,

    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn init(client: Client, args: Args, denom: &str) {
    info!("Starting contract instantiation!");

    let memo = args
        .memo
        .unwrap_or_else(|| "contract instantiation".to_owned());
    let label = args
        .label
        .unwrap_or_else(|| "Nym mixnet smart contract".to_owned());

    // by default we make ourselves an admin, let me know if you don't like that behaviour
    let opts = Some(InstantiateOptions {
        funds: vec![],
        admin: Some(client.address().clone()),
    });

    // currently (as of time of writing this - look at commit time)
    // the EmptyMsg{} argument is equivalent to `--init-message='{}'`
    let res = if let Some(raw_msg) = args.init_message {
        let msg: serde_json::Value =
            serde_json::from_str(&raw_msg).expect("failed to parse init message");

        client
            .instantiate(args.code_id, &msg, label, memo, opts, None)
            .await
            .expect("failed to instantiate the contract!")
    } else {
        let address = client.address().to_string();
        client
            .instantiate(
                args.code_id,
                &InstantiateMsg {
                    rewarding_validator_address: address,
                    mixnet_denom: denom.to_string(),
                },
                label,
                memo,
                opts,
                None,
            )
            .await
            .expect("failed to instantiate the contract!")
    };

    info!("Init result: {:?}", res);

    // I can only assume ansible will only care about contract address, so let's output it on separate line
    // to stdout for easier parsing from ansible
    println!("{}", res.contract_address)
}
