// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub profit_percent: u8,
}

pub async fn update_profit_percent(args: Args, client: SigningClient) {
    info!("Update mix node profit percent - get those rewards!");

    //profit percent between 1-100
    let res = client
        .update_mixnode_config(args.profit_percent, None)
        .await
        .expect("updating mix-node profit percent");

    info!("profit percentage updated: {:?}", res)
}
