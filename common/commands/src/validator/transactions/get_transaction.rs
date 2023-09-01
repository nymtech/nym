// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use std::str::FromStr;

use crate::context::QueryClient;
use crate::utils::show_error;
use cosmrs::tendermint::Hash;
use nym_validator_client::nyxd::CosmWasmClient;
use serde_json::json;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The transaction hash")]
    pub tx_hash: String,
}

pub async fn get(args: Args, client: &QueryClient) {
    let hash = Hash::from_str(&args.tx_hash).expect("could not parse transaction hash");

    match client.get_tx(hash).await {
        Ok(res) => {
            println!("{}", json!(res))
        }
        Err(e) => show_error(e),
    }
}
