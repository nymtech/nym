// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use cosmrs::rpc::query::Query;
use nym_validator_client::nyxd::CosmWasmClient;
use serde_json::json;

use crate::context::QueryClient;
use crate::utils::show_error;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The query to execute")]
    pub query: String,
}

pub async fn query(args: Args, client: &QueryClient) {
    match Query::from_str(&args.query) {
        Ok(query) => match client.search_tx(query).await {
            Ok(res) => {
                println!("{}", json!(res))
            }
            Err(e) => show_error(e),
        },
        Err(e) => show_error(e),
    }
}
