// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

use crate::context::QueryClient;
use crate::utils::show_error;
use serde_json::json;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The block height")]
    pub height: u32,
}

pub async fn query_for_block(args: Args, client: &QueryClient) {
    match client.get_block(Some(args.height)).await {
        Ok(res) => {
            println!("{}", json!(res))
        }
        Err(e) => show_error(e),
    }
}
