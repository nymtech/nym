// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

use crate::context::QueryClient;
use crate::utils::show_error;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn query_current_block_height(client: &QueryClient) {
    match client.get_current_block_height().await {
        Ok(res) => {
            println!("Current block height:\n{}", res.value())
        }
        Err(e) => show_error(e),
    }
}
