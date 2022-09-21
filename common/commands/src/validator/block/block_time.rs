// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

use crate::context::QueryClient;
use crate::utils::show_error;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The block height")]
    pub height: u32,
}

pub async fn query_for_block_time(args: Args, client: &QueryClient) {
    match client.get_block_timestamp(Some(args.height)).await {
        Ok(res) => {
            println!("{}", res.to_rfc3339())
        }
        Err(e) => show_error(e),
    }
}
