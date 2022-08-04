// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use validator_client::nymd::VestingSigningClient;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub profit_percent: u8,

    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn vesting_update_profit_percent(client: Client, args: Args) {
    info!("Update vesting mix node profit percent - get those rewards!");

    //profit percent between 1-100
    let res = client
        .vesting_update_mixnode_config(args.profit_percent, None)
        .await
        .expect("updating vesting mix-node profit percent");

    info!("profit percentage updated: {:?}", res)
}
